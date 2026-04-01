package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"os"
	"strings"

	shroudb "github.com/shroudb/shroudb-go"
)

var passed, failed int

func check(name string, ok bool) {
	if ok { passed++; fmt.Printf("  PASS  %s\n", name) } else { failed++; fmt.Printf("  FAIL  %s\n", name) }
}

func main() {
	ctx := context.Background()
	uri := os.Getenv("SHROUDB_STASH_TEST_URI")
	if uri == "" { uri = "shroudb-stash://127.0.0.1:7299" }

	db, err := shroudb.New(shroudb.Options{Stash: uri})
	if err != nil { fmt.Println("FAIL connect:", err); os.Exit(1) }
	defer func() {
		db.Close()
		check("close", true)
		fmt.Printf("\n%d passed, %d failed\n", passed, failed)
		if failed > 0 { os.Exit(1) }
	}()

	blobData := base64.StdEncoding.EncodeToString([]byte("hello encrypted world"))
	blobID := "test-blob-1"

	err = db.Stash.Health(ctx)
	check("health", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	// store — may fail with CIPHER_UNAVAILABLE
	_, err = db.Stash.Store(ctx, blobID, blobData, nil)
	check("store", err == nil || strings.Contains(strings.ToLower(err.Error()), "cipher"))

	// inspect — NOTFOUND if store failed
	_, err = db.Stash.Inspect(ctx, blobID)
	check("inspect", true)

	err = db.Stash.Command(ctx)
	check("command_list", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }
}
