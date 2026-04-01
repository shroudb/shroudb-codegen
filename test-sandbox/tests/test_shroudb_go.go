// ShrouDB core unified SDK Go integration test.
package main

import (
	"context"
	"fmt"
	"os"
	"strings"

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
	result, err := db.Shroudb.Get(ctx, "test-ns", "test-key", nil, nil)
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
	_, err = db.Shroudb.Get(ctx, "test-ns", "test-key", nil, nil)
	check("error_after_delete", err != nil)
}
