// ShrouDB Chronicle unified SDK Go integration test.
package main

import (
	"context"
	"fmt"
	"os"
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
	uri := os.Getenv("SHROUDB_CHRONICLE_TEST_URI")
	if uri == "" {
		uri = "shroudb-chronicle://127.0.0.1:6899"
	}

	db, err := shroudb.New(shroudb.Options{Chronicle: uri})
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

	// Handshake sanity — every engine must answer HELLO.
	{
		h, err := db.Chronicle.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "chronicle")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	// 1. Health
	_, err = db.Chronicle.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. Ingest (push a test event as JSON string)
	eventPayload := map[string]interface{}{
		"id":          "test-event-1",
		"engine":      "shroudb",
		"operation":   "sdk_test",
		"resource":    "test/resource",
		"result":      "ok",
		"actor":       "user:test@example.com",
		"timestamp":   time.Now().Unix(),
		"duration_ms": 1,
	}
	_, err = db.Chronicle.Ingest(ctx, eventPayload)
	check("ingest", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 3. Query (retrieve events)
	_, err = db.Chronicle.Query(ctx, nil)
	check("query", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. Count
	_, err = db.Chronicle.Count(ctx, nil)
	check("count", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. IngestBatch
	batchPayload := []map[string]any{
		{
			"id":          "batch-event-1",
			"engine":      "shroudb",
			"operation":   "sdk_test_batch",
			"resource":    "test/batch",
			"result":      "ok",
			"actor":       "user:batch@example.com",
			"timestamp":   time.Now().Unix(),
			"duration_ms": 2,
		},
		{
			"id":          "batch-event-2",
			"engine":      "shroudb",
				"operation":   "sdk_test_batch",
				"resource":    "test/batch",
				"result":      "ok",
				"actor":       "user:batch@example.com",
				"timestamp":   time.Now().Unix(),
				"duration_ms": 3,
			},
	}
	batchResult, err := db.Chronicle.IngestBatch(ctx, batchPayload)
	check("ingest_batch", err == nil && batchResult != nil && batchResult.Ingested >= 2)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 6. Actors
	_, err = db.Chronicle.Actors(ctx, nil)
	check("actors", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 7. Hotspots
	_, err = db.Chronicle.Hotspots(ctx, nil)
	check("hotspots", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
}
