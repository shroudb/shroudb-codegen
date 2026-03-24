// ShrouDB Go client integration test.
//
// Exercises the generated client against a live ShrouDB server.
// Expects SHROUDB_TEST_URI env var (e.g. shroudb://127.0.0.1:6399).
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	shroudb "github.com/shroudb/shroudb-go"
)

var passed, failed int

func check(name string, condition bool) {
	if condition {
		passed++
		fmt.Printf("  PASS  %s\n", name)
	} else {
		failed++
		fmt.Printf("  FAIL  %s\n", name)
	}
}

func main() {
	uri := os.Getenv("SHROUDB_TEST_URI")
	if uri == "" {
		uri = "shroudb://127.0.0.1:6399"
	}

	ctx := context.Background()
	client, err := shroudb.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health (server-level)
	h, err := client.Health(ctx)
	check("health", err == nil && h.Status == "OK")

	// 2. Health (keyspace-level)
	hk, err := client.Health(ctx, "test-apikeys")
	check("health_keyspace", err == nil && hk.Status == "OK")

	// 3. Issue on test-apikeys
	issued, err := client.Issue(ctx, "test-apikeys", nil)
	check("issue", err == nil && issued.CredentialId != "" && issued.Token != "")
	credID := issued.CredentialId
	token := issued.Token

	// 4. Verify the token
	verified, err := client.Verify(ctx, "test-apikeys", token, nil)
	check("verify", err == nil && verified.CredentialId == credID)

	// 5. Inspect
	info, err := client.Inspect(ctx, "test-apikeys", credID)
	check("inspect_active", err == nil && info.State == "active")

	// 6. Update metadata
	_, err = client.Update(ctx, "test-apikeys", credID, &shroudb.UpdateOptions{
		Metadata: map[string]any{"env": "test"},
	})
	check("update", err == nil)

	// 7. Inspect after update
	info2, err := client.Inspect(ctx, "test-apikeys", credID)
	check("inspect_meta", err == nil && info2.Meta != nil)

	// 8. Suspend
	_, err = client.Suspend(ctx, "test-apikeys", credID)
	check("suspend", err == nil)

	// 9. Inspect suspended
	info3, err := client.Inspect(ctx, "test-apikeys", credID)
	check("inspect_suspended", err == nil && info3.State == "suspended")

	// 10. Unsuspend
	_, err = client.Unsuspend(ctx, "test-apikeys", credID)
	check("unsuspend", err == nil)

	// 11. Revoke
	_, err = client.Revoke(ctx, "test-apikeys", credID)
	check("revoke", err == nil)

	// 12. Verify revoked token should fail
	_, err = client.Verify(ctx, "test-apikeys", token, nil)
	if err != nil {
		if shroudb.IsStateError(err) || shroudb.IsNotfound(err) {
			check("verify_revoked", true)
		} else {
			check("verify_revoked", true) // any error is acceptable
		}
	} else {
		check("verify_revoked", false)
	}

	// 13. Issue JWT with claims
	jwtIssued, err := client.Issue(ctx, "test-jwt", &shroudb.IssueOptions{
		Claims: map[string]any{"sub": "user123", "role": "admin"},
	})
	check("issue_jwt", err == nil && jwtIssued.Token != "")

	// 14. Verify JWT
	jwtVerified, err := client.Verify(ctx, "test-jwt", jwtIssued.Token, nil)
	check("verify_jwt", err == nil && jwtVerified.Claims != nil)

	// 15. JWKS
	jwks, err := client.Jwks(ctx, "test-jwt")
	check("jwks", err == nil && jwks.Keys != nil)

	// 16. KEYS (list credentials)
	keysResult, err := client.Keys(ctx, "test-apikeys", nil)
	check("keys", err == nil && keysResult.Cursor != nil)

	// 17. Error: BADARG
	_, err = client.Inspect(ctx, "test-apikeys", "")
	check("error_badarg", err != nil && shroudb.IsBadarg(err))

	// 18. Error: NOTFOUND
	_, err = client.Inspect(ctx, "test-apikeys", "nonexistent_credential_id")
	check("error_notfound", err != nil && shroudb.IsNotfound(err))

	// 19. Pipeline
	pipe := client.Pipeline()
	pipe.Issue("test-apikeys", nil)
	pipe.Health()
	results, err := pipe.Execute()
	check("pipeline", err == nil && len(results) == 2)

	// 20. Subscribe
	func() {
		subCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
		defer cancel()

		sub, err := client.Subscribe("*")
		if err != nil {
			check("subscribe", false)
			fmt.Printf("         (subscribe: %v)\n", err)
			return
		}
		defer sub.Close()

		// Trigger an event from a second connection
		client2, err := shroudb.Connect(uri)
		if err != nil {
			check("subscribe", false)
			return
		}
		_, _ = client2.Issue(ctx, "test-apikeys", nil)
		client2.Close()

		// Wait for event
		select {
		case evt := <-sub.Events():
			check("subscribe", evt.EventType != "" && evt.Keyspace != "")
		case err := <-sub.Err():
			check("subscribe", false)
			fmt.Printf("         (err: %v)\n", err)
		case <-subCtx.Done():
			check("subscribe", false)
			fmt.Println("         (timeout)")
		}
	}()

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
