// ShrouDB core unified SDK Go integration test.
package main

import (
	"context"
	"fmt"
	"os"
	"strings"
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
	uri := os.Getenv("SHROUDB_SHROUDB_TEST_URI")
	if uri == "" {
		uri = "shroudb://127.0.0.1:6399"
	}

	db, err := shroudb.New(shroudb.Options{Shroudb: uri})
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
	_, err = db.Shroudb.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. Namespace create (required before PUT/GET in v1)
	err = db.Shroudb.NamespaceCreate(ctx, "test-ns", nil)
	if err != nil && strings.Contains(strings.ToLower(err.Error()), "exists") {
		check("namespace_create", true)
	} else if err == nil {
		check("namespace_create", true)
	} else {
		check("namespace_create", false)
		fmt.Printf("    error: %v\n", err)
	}

	// 3. PUT
	_, err = db.Shroudb.Put(ctx, "test-ns", "test-key", "test-value", nil)
	check("put", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. GET
	result, err := db.Shroudb.Get(ctx, "test-ns", "test-key", false, nil)
	check("get", err == nil && result != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. DELETE
	_, err = db.Shroudb.Delete(ctx, "test-ns", "test-key")
	check("delete", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 6. Error: GET after delete
	_, err = db.Shroudb.Get(ctx, "test-ns", "test-key", false, nil)
	check("error_after_delete", err != nil)

	// 7. PIPELINE: atomic batch of commands on one round-trip.
	results, err := db.Shroudb.Pipeline(ctx, [][]string{
		{"PUT", "test-ns", "pipe-k1", "v1"},
		{"PUT", "test-ns", "pipe-k2", "v2"},
		{"GET", "test-ns", "pipe-k1"},
	}, "")
	check("pipeline_returns_array", err == nil && results != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
	if err == nil {
		check("pipeline_length", len(results) == 3)
		val, _ := results[2]["value"].(string)
		check("pipeline_get_value", val == "v1")
	}

	// 8. PIPELINE idempotency: same request_id returns cached result.
	rid := fmt.Sprintf("test-idempotency-%d", time.Now().UnixNano())
	first, err1 := db.Shroudb.Pipeline(ctx,
		[][]string{{"PUT", "test-ns", "pipe-idem", "first"}}, rid)
	second, err2 := db.Shroudb.Pipeline(ctx,
		[][]string{{"PUT", "test-ns", "pipe-idem", "second"}}, rid)
	if err1 == nil && err2 == nil && len(first) == 1 && len(second) == 1 {
		firstVersion, _ := first[0]["version"].(int)
		secondVersion, _ := second[0]["version"].(int)
		check("pipeline_idempotent_replay", firstVersion == secondVersion)
		current, err := db.Shroudb.Get(ctx, "test-ns", "pipe-idem", false, nil)
		if err == nil && current != nil {
			check("pipeline_idempotent_value_unchanged", current.Value == "first")
		} else {
			check("pipeline_idempotent_value_unchanged", false)
		}
	} else {
		check("pipeline_idempotency", false)
		if err1 != nil {
			fmt.Printf("    error1: %v\n", err1)
		}
		if err2 != nil {
			fmt.Printf("    error2: %v\n", err2)
		}
	}
}
