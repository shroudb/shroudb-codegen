// ShrouDB Scroll unified SDK Go integration test.
package main

import (
	"context"
	"encoding/base64"
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
	uri := os.Getenv("SHROUDB_SCROLL_TEST_URI")
	if uri == "" {
		uri = "shroudb-scroll://127.0.0.1:7200"
	}

	db, err := shroudb.New(shroudb.Options{Scroll: uri})
	if err != nil {
		fmt.Println("FAIL connect:", err)
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

	log := fmt.Sprintf("sandbox-log-go-%d", time.Now().Unix()%100000)
	group := "workers"

	// Handshake sanity — every engine must answer HELLO.
	{
		h, err := db.Scroll.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "scroll")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	_, err = db.Scroll.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	err = db.Scroll.Ping(ctx)
	check("ping", err == nil)

	// append (creates the log on first call)
	firstPayload := base64.StdEncoding.EncodeToString([]byte("hello scroll"))
	r, err := db.Scroll.Append(ctx, log, firstPayload, nil)
	check("append: first", err == nil && r != nil && r.Offset == 0)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	r, err = db.Scroll.Append(ctx, log, base64.StdEncoding.EncodeToString([]byte("second")), nil)
	check("append: second", err == nil && r != nil && r.Offset == 1)

	// read
	rr, err := db.Scroll.Read(ctx, log, 0, 10)
	if err == nil && rr != nil {
		check("read: count", len(rr.Entries) == 2)
		decoded, decErr := base64.StdEncoding.DecodeString(rr.Entries[0].PayloadB64)
		check("read: payload roundtrip", decErr == nil && string(decoded) == "hello scroll")
	} else {
		check("read: count", false)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	}

	_, err = db.Scroll.CreateGroup(ctx, log, group, "earliest")
	check("create_group", err == nil)

	rg, err := db.Scroll.ReadGroup(ctx, log, group, "reader-1", 10)
	if err == nil && rg != nil {
		check("read_group: count", len(rg.Entries) == 2)
	} else {
		check("read_group: count", false)
	}

	_, e1 := db.Scroll.Ack(ctx, log, group, 0)
	_, e2 := db.Scroll.Ack(ctx, log, group, 1)
	check("ack", e1 == nil && e2 == nil)

	info, err := db.Scroll.LogInfo(ctx, log)
	if err == nil && info != nil {
		check("log_info: entries_minted", info.EntriesMinted == 2)
		hasGroup := false
		for _, g := range info.Groups {
			if g == group {
				hasGroup = true
				break
			}
		}
		check("log_info: has group", hasGroup)
	} else {
		check("log_info", false)
	}

	gi, err := db.Scroll.GroupInfo(ctx, log, group)
	if err == nil && gi != nil {
		check("group_info: cursor", gi.LastDeliveredOffset == 1)
		check("group_info: pending_count", gi.PendingCount == 0)
	} else {
		check("group_info", false)
	}

	_, err = db.Scroll.DeleteGroup(ctx, log, group)
	check("delete_group", err == nil)

	_, err = db.Scroll.DeleteLog(ctx, log)
	check("delete_log", err == nil)
}
