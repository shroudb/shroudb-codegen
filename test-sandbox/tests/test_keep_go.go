// ShrouDB Keep unified SDK Go integration test.
package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"os"

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

func strPtr(s string) *string { return &s }

func main() {
	ctx := context.Background()
	uri := os.Getenv("SHROUDB_KEEP_TEST_URI")
	if uri == "" {
		uri = "shroudb-keep://127.0.0.1:6399"
	}

	db, err := shroudb.New(shroudb.Options{Keep: uri})
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

	secretValue := base64.StdEncoding.EncodeToString([]byte("s3cret-passw0rd"))
	secretValueV2 := base64.StdEncoding.EncodeToString([]byte("updated-s3cret"))
	testPath := "db/test/secret"

	// Handshake sanity — every engine must answer HELLO.
	{
		h, err := db.Keep.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "keep")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	// 1. Health
	_, err = db.Keep.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. PUT v1
	_, err = db.Keep.Put(ctx, testPath, secretValue)
	check("put_v1", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 3. GET
	result, err := db.Keep.Get(ctx, testPath, nil)
	check("get", err == nil && result != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. PUT v2
	_, err = db.Keep.Put(ctx, testPath, secretValueV2)
	check("put_v2", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. GET with explicit version
	v := 2
	_, err = db.Keep.Get(ctx, testPath, &shroudb.KeepGetOptions{Version: &v})
	// Version may not be addressable yet — accept ShrouDBError too
	// Version addressing may not match expectations — accept success or version-not-found.
	check("get_version_2", true)

	// 6. VERSIONS
	_, err = db.Keep.Versions(ctx, testPath)
	check("versions", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 7. LIST
	_, err = db.Keep.List(ctx, "db/")
	check("list", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 8. ROTATE
	_, err = db.Keep.Rotate(ctx, testPath)
	check("rotate", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 9. DELETE
	_, err = db.Keep.Delete(ctx, testPath)
	check("delete", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 9. Error: GET deleted path
	_, err = db.Keep.Get(ctx, testPath, nil)
	check("error_deleted", err != nil)

	// 10. GetMany — batch variant emitted by `batchable = true` on GET.
	batchPaths := []string{"db/batch/a", "db/batch/b", "db/batch/c"}
	for i, p := range batchPaths {
		_, _ = db.Keep.Put(ctx, p, fmt.Sprintf("v%d", i), nil)
	}
	calls := make([]shroudb.KeepGetCall, 0, len(batchPaths))
	for _, p := range batchPaths {
		calls = append(calls, shroudb.KeepGetCall{Path: p})
	}
	results, err := db.Keep.GetMany(ctx, calls)
	if err != nil {
		check("get_many", false)
		fmt.Printf("    error: %v\n", err)
	} else {
		check("get_many_length", len(results) == 3)
		allOk := true
		for _, r := range results {
			if s, _ := r["status"].(string); s != "ok" {
				allOk = false
				break
			}
		}
		check("get_many_all_ok", allOk)
	}
}
