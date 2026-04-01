// ShrouDB Forge unified SDK Go integration test.
package main

import (
	"context"
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

func main() {
	ctx := context.Background()
	uri := os.Getenv("SHROUDB_FORGE_TEST_URI")
	if uri == "" {
		uri = "shroudb-forge://127.0.0.1:6699"
	}

	db, err := shroudb.New(shroudb.Options{Forge: uri})
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

	// 1. Health via ca_list (forge has no RESP3 HEALTH command)
	_, err = db.Forge.CaList(ctx)
	check("health_via_ca_list", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. CaInfo
	_, err = db.Forge.CaInfo(ctx, "test-ca")
	check("ca_info", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 3. CaList
	_, err = db.Forge.CaList(ctx)
	check("ca_list", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. Issue certificate
	cert, err := db.Forge.Issue(ctx, "test-ca", "CN=test.example.com", "server", nil)
	check("issue", err == nil && cert != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	serial := ""
	if cert != nil {
		serial = cert.Serial
	}

	// 5. Inspect (use serial from issue)
	if serial != "" {
		inspectResult, err := db.Forge.Inspect(ctx, "test-ca", serial)
		check("inspect", err == nil && inspectResult != nil && inspectResult.Serial == serial)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	} else {
		check("inspect", false)
		fmt.Println("    skipped: no serial from issue")
	}

	// 6. ListCerts
	lcResult, err := db.Forge.ListCerts(ctx, "test-ca", nil)
	check("list_certs", err == nil && lcResult != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 7. Revoke (use serial from issue)
	if serial != "" {
		_, err = db.Forge.Revoke(ctx, "test-ca", serial, nil)
		check("revoke", err == nil)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	} else {
		check("revoke", false)
		fmt.Println("    skipped: no serial from issue")
	}
}
