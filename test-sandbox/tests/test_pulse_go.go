// ShrouDB Pulse Go client integration test.
package main

import (
	"fmt"
	"os"

	shroudb_pulse "github.com/shroudb/shroudb-pulse-go"
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
	uri := os.Getenv("SHROUDB_PULSE_TEST_URI")
	if uri == "" {
		uri = "shroudb-pulse://127.0.0.1:6999"
	}

	client, err := shroudb_pulse.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health
	err = client.Health("")
	check("health", err == nil)

	// 2. INGEST (push a test event)
	err = client.Ingest(&shroudb_pulse.IngestRequest{
		Source:    "test-source",
		EventType: "test.event",
		Data:      map[string]interface{}{"message": "hello from integration test"},
	})
	check("ingest", err == nil)

	// 3. QUERY (retrieve the event)
	_, err = client.Query(&shroudb_pulse.QueryOptions{Source: "test-source"})
	check("query", err == nil)

	// 4. COUNT
	_, err = client.Count()
	check("count", err == nil)

	// 5. SOURCE_LIST
	_, err = client.SourceList()
	check("source_list", err == nil)

	// 6. SOURCE_STATUS
	_, err = client.SourceStatus("test-source")
	check("source_status", err == nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
