// ShrouDB Sentry Go client integration test.
package main

import (
	"fmt"
	"os"

	shroudb_sentry "github.com/shroudb/shroudb-sentry-go"
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
	uri := os.Getenv("SHROUDB_SENTRY_TEST_URI")
	if uri == "" {
		uri = "shroudb-sentry://127.0.0.1:6699"
	}

	client, err := shroudb_sentry.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health
	err = client.Health("")
	check("health", err == nil)

	// 2. POLICY_LIST
	_, err = client.PolicyList()
	check("policy_list", err == nil)

	// 3. EVALUATE
	_, err = client.Evaluate(&shroudb_sentry.EvaluateRequest{
		Principal: map[string]string{"role": "admin"},
		Resource:  map[string]string{"type": "document"},
		Action:    map[string]string{"name": "read"},
	})
	check("evaluate", err == nil)

	// 4. KEY_INFO
	_, err = client.KeyInfo()
	check("key_info", err == nil)

	// 5. Error: POLICY_INFO nonexistent
	_, err = client.PolicyInfo("nonexistent")
	check("error_notfound", err != nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
