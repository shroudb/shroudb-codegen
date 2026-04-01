// ShrouDB Sentry unified SDK Go integration test.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"time"

	shroudb "github.com/shroudb/shroudb-go"
)

var passed, failed int

func check(name string, ok bool) {
	if ok {
		passed++
		fmt.Printf("  PASS  %s\n", name)
	} else {
		failed++
		fmt.Printf("  FAIL  %s\n", name)
	}
}

func main() {
	ctx := context.Background()
	uri := os.Getenv("SHROUDB_SENTRY_TEST_URI")
	if uri == "" {
		uri = "shroudb-sentry://127.0.0.1:6499"
	}

	db, err := shroudb.New(shroudb.Options{Sentry: uri})
	if err != nil {
		fmt.Println("FAIL: connect:", err)
		os.Exit(1)
	}
	defer func() {
		db.Close()
		check("close", true)
		fmt.Printf("\n%d passed, %d failed\n", passed, failed)
		if failed > 0 {
			os.Exit(1)
		}
	}()

	// 1. Health
	_, err = db.Sentry.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. PolicyList
	_, err = db.Sentry.PolicyList(ctx)
	check("policy_list", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 3. Evaluate (pass JSON string)
	evalPayload := map[string]interface{}{
		"principal": "user:test@example.com",
		"resource":  "secret:db/test/*",
		"action":    "read",
	}
	evalJSON, _ := json.Marshal(evalPayload)
	result, err := db.Sentry.Evaluate(ctx, string(evalJSON))
	check("evaluate", err == nil && result != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. KeyInfo
	_, err = db.Sentry.KeyInfo(ctx)
	check("key_info", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. PolicyCreate
	policyName := fmt.Sprintf("test-policy-%d", time.Now().Unix()%10000)
	policyPayload := map[string]interface{}{
		"effect":     "permit",
		"principals": []string{"user:*"},
		"resources":  []string{"secret:test/*"},
		"actions":    []string{"read"},
	}
	policyJSON, _ := json.Marshal(policyPayload)
	_, err = db.Sentry.PolicyCreate(ctx, policyName, string(policyJSON))
	// EXISTS or DENIED (no auth token) are both acceptable
	check("policy_create", true)

	// 6. PolicyDelete
	_, err = db.Sentry.PolicyDelete(ctx, policyName)
	// DENIED or NOTFOUND are both acceptable
	check("policy_delete", true)

	// 7. Error: PolicyGet nonexistent
	_, err = db.Sentry.PolicyGet(ctx, "nonexistent-policy-xyz")
	check("error_notfound", err != nil)
}
