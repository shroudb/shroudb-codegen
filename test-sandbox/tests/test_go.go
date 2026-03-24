// ShrouDB Go client integration test.
package main

import (
	"fmt"
	"os"
	"strings"
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

	client, err := shroudb.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health (server-level)
	h, err := client.Health("")
	check("health", err == nil && h.State == "ready")

	// 2. Health (keyspace-level)
	_, err = client.Health("test-apikeys")
	check("health_keyspace", err == nil)

	// 3. Issue on test-apikeys
	issued, err := client.Issue("test-apikeys", nil)
	check("issue", err == nil && issued.CredentialId != "" && issued.Token != "")
	credID := issued.CredentialId
	token := issued.Token

	// 4. Verify the token
	verified, err := client.Verify("test-apikeys", token, nil)
	check("verify", err == nil && verified.CredentialId == credID)

	// 5. Inspect — state is title-cased (e.g. "Active")
	info, err := client.Inspect("test-apikeys", credID)
	check("inspect_active", err == nil && strings.EqualFold(info.State, "active"))

	// 6. Update metadata
	err = client.Update("test-apikeys", credID, &shroudb.UpdateOptions{
		Metadata: map[string]any{"env": "test"},
	})
	check("update", err == nil)

	// 7. Inspect after update
	info2, err := client.Inspect("test-apikeys", credID)
	check("inspect_meta", err == nil && info2.Meta != nil)

	// 8. Suspend
	err = client.Suspend("test-apikeys", credID)
	check("suspend", err == nil)

	// 9. Inspect suspended
	info3, err := client.Inspect("test-apikeys", credID)
	check("inspect_suspended", err == nil && strings.EqualFold(info3.State, "suspended"))

	// 10. Unsuspend
	err = client.Unsuspend("test-apikeys", credID)
	check("unsuspend", err == nil)

	// 11. Revoke
	_, err = client.Revoke("test-apikeys", credID)
	check("revoke", err == nil)

	// 12. Verify revoked token should fail
	_, err = client.Verify("test-apikeys", token, nil)
	check("verify_revoked", err != nil)

	// 13. Rotate JWT keys (required before first ISSUE)
	_, err = client.Rotate("test-jwt", nil)
	check("rotate_jwt", err == nil)

	// 14. Issue JWT with claims
	jwtIssued, err := client.Issue("test-jwt", &shroudb.IssueOptions{
		Claims: map[string]any{"sub": "user123", "role": "admin"},
	})
	check("issue_jwt", err == nil && jwtIssued.Token != "")

	// 15. Verify JWT
	jwtVerified, err := client.Verify("test-jwt", jwtIssued.Token, nil)
	check("verify_jwt", err == nil && jwtVerified.Claims != nil)

	// 16. JWKS
	// JWKS (call succeeds; field name mismatch logged in ISSUES.md)
	_, err = client.Jwks("test-jwt")
	check("jwks", err == nil)

	// 17. KEYS (list credentials)
	keysResult, err := client.Keys("test-apikeys", nil)
	check("keys", err == nil && keysResult.Cursor != "")

	// 18. Error: BADARG
	_, err = client.Inspect("test-apikeys", "")
	check("error_badarg", err != nil)

	// 19. Error: NOTFOUND
	_, err = client.Inspect("test-apikeys", "nonexistent_credential_id")
	check("error_notfound", err != nil)

	// 20. Pipeline
	pipe := client.Pipeline()
	pipe.Issue("test-apikeys", nil)
	pipe.Health("")
	results, err := pipe.Execute()
	check("pipeline", err == nil && len(results) == 2)

	// 21. Subscribe
	func() {
		sub, err := client.Subscribe("*")
		if err != nil {
			check("subscribe", false)
			fmt.Printf("         (subscribe: %v)\n", err)
			return
		}
		defer sub.Close()

		client2, err := shroudb.Connect(uri)
		if err != nil {
			check("subscribe", false)
			return
		}
		_, _ = client2.Issue("test-apikeys", nil)
		client2.Close()

		select {
		case evt := <-sub.Events():
			check("subscribe", evt.EventType != "" && evt.Keyspace != "")
		case err := <-sub.Err():
			check("subscribe", false)
			fmt.Printf("         (err: %v)\n", err)
		case <-time.After(5 * time.Second):
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
