// ShrouDB Courier unified SDK Go integration test.
package main

import (
	"context"
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
	uri := os.Getenv("SHROUDB_COURIER_TEST_URI")
	if uri == "" {
		uri = "shroudb-courier://127.0.0.1:6899"
	}

	db, err := shroudb.New(shroudb.Options{Courier: uri})
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
		h, err := db.Courier.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "courier")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	// 1. Health
	_, err = db.Courier.Health(ctx)
	check("health", err == nil)

	// 2. ChannelList
	_, err = db.Courier.ChannelList(ctx)
	check("channel_list", err == nil)

	// 3. ChannelCreate
	channelName := fmt.Sprintf("test-channel-%d", time.Now().Unix()%10000)
	config := `{"url":"https://example.com/webhook"}`
	ccResult, err := db.Courier.ChannelCreate(ctx, channelName, "webhook", config)
	if err != nil && strings.Contains(strings.ToLower(err.Error()), "exists") {
		check("channel_create", true)
	} else {
		check("channel_create", err == nil && ccResult != nil && ccResult.Name == channelName)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	}

	// 4. ChannelDelete
	_, err = db.Courier.ChannelDelete(ctx, channelName)
	check("channel_delete", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
}
