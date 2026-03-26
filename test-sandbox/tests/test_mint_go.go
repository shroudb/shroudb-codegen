// ShrouDB Mint Go client integration test.
package main

import (
	"fmt"
	"os"

	shroudb_mint "github.com/shroudb/shroudb-mint-go"
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
	uri := os.Getenv("SHROUDB_MINT_TEST_URI")
	if uri == "" {
		uri = "shroudb-mint://127.0.0.1:6599"
	}

	client, err := shroudb_mint.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health
	err = client.Health("")
	check("health", err == nil)

	// 2. CA_INFO test-ca
	_, err = client.CaInfo("test-ca")
	check("ca_info", err == nil)

	// 3. CA_LIST
	_, err = client.CaList()
	check("ca_list", err == nil)

	// 4. ISSUE test-ca with profile server
	cert, err := client.Issue("test-ca", "CN=test-svc", &shroudb_mint.IssueOptions{Profile: "server"})
	check("issue", err == nil && cert != nil)

	serial := ""
	if cert != nil {
		serial = cert.Serial
		if serial == "" {
			serial = cert.SerialNumber
		}
	}

	// 5. INSPECT test-ca <serial>
	if serial != "" {
		_, err = client.Inspect("test-ca", serial)
		check("inspect", err == nil)
	} else {
		check("inspect", false)
	}

	// 6. LIST_CERTS test-ca
	_, err = client.ListCerts("test-ca")
	check("list_certs", err == nil)

	// 7. REVOKE test-ca <serial>
	if serial != "" {
		_, err = client.Revoke("test-ca", serial)
		check("revoke", err == nil)
	} else {
		check("revoke", false)
	}

	// 8. CA_ROTATE test-ca FORCE
	_, err = client.CaRotate("test-ca", &shroudb_mint.CaRotateOptions{Force: "true"})
	check("ca_rotate", err == nil)

	// 9. CA_EXPORT test-ca
	_, err = client.CaExport("test-ca")
	check("ca_export", err == nil)

	// 10. Error: CA_INFO nonexistent
	_, err = client.CaInfo("nonexistent")
	check("error_notfound", err != nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
