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
	blobID := fmt.Sprintf("test-blob-go-%d", os.Getpid())

	// Handshake sanity — every engine must answer HELLO.
	{
		h, err := db.Stash.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "stash")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	err = db.Stash.Health(ctx)
	check("health", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	_, err = db.Stash.Store(ctx, blobID, blobData, nil)
	check("store", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	_, err = db.Stash.Inspect(ctx, blobID)
	check("inspect", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	err = db.Stash.Retrieve(ctx, blobID)
	check("retrieve", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	_, err = db.Stash.Revoke(ctx, blobID, nil)
	check("revoke_soft", err == nil)
	if err != nil { fmt.Printf("    error: %v\n", err) }

	err = db.Stash.Retrieve(ctx, blobID)
	check("error_after_revoke", err != nil)

	// Hard revoke
	blobID2 := blobID + "-shred"
	_, err = db.Stash.Store(ctx, blobID2, blobData, nil)
	if err == nil {
		_, err = db.Stash.Revoke(ctx, blobID2, nil)
		check("revoke_hard", err == nil)
		if err != nil { fmt.Printf("    error: %v\n", err) }

		err = db.Stash.Retrieve(ctx, blobID2)
		check("error_after_shred", err != nil)
	} else {
		check("revoke_hard", false)
		fmt.Printf("    error: %v\n", err)
		check("error_after_shred", false)
	}
}
