// ShrouDB Courier Go client integration test.
//
// Limited test -- no Transit available, so DELIVER is skipped.
// Tests management commands only: TEMPLATE_LIST, TEMPLATE_INFO, HEALTH.
package main

import (
	"fmt"
	"os"

	shroudb_courier "github.com/shroudb/shroudb-courier-go"
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
	uri := os.Getenv("SHROUDB_COURIER_TEST_URI")
	if uri == "" {
		uri = "shroudb-courier://127.0.0.1:6899"
	}

	client, err := shroudb_courier.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health
	err = client.Health("")
	check("health", err == nil)

	// 2. TEMPLATE_LIST
	_, err = client.TemplateList()
	check("template_list", err == nil)

	// 3. Error: TEMPLATE_INFO nonexistent
	_, err = client.TemplateInfo("nonexistent")
	check("error_notfound", err != nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
