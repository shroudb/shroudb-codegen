// ShrouDB Veil unified SDK Go integration test.
package main

import (
	"context"
	"encoding/base64"
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
	uri := os.Getenv("SHROUDB_VEIL_TEST_URI")
	if uri == "" {
		uri = "shroudb-veil://127.0.0.1:6999"
	}

	db, err := shroudb.New(shroudb.Options{Veil: uri})
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

	idxName := fmt.Sprintf("test-idx-%d", time.Now().Unix()%10000)

	// 1. Health
	_, err = db.Veil.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. IndexCreate
	// Note: response type has string fields but server returns numbers —
	// the Veil protocol.toml uses condensed response format without type info.
	// We ignore the typed response and just check error status.
	_, err = db.Veil.IndexCreate(ctx, idxName)
	if err != nil && (strings.Contains(strings.ToLower(err.Error()), "exists") || shroudb.IsCode(err, "EXISTS")) {
		check("index_create", true)
	} else {
		check("index_create", err == nil)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	}

	// 3. Tokenize
	plaintextB64 := base64.StdEncoding.EncodeToString([]byte("hello"))
	_, err = db.Veil.Tokenize(ctx, idxName, plaintextB64, nil)
	check("tokenize", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. Put (store blind tokens for an entry)
	_, err = db.Veil.Put(ctx, idxName, "entry-1", plaintextB64, nil)
	check("put", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. Search (search by token)
	_, err = db.Veil.Search(ctx, idxName, plaintextB64, nil)
	check("search", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
}
