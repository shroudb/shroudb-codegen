// ShrouDB Keep Go client integration test.
package main

import (
	"encoding/base64"
	"fmt"
	"os"

	shroudb_keep "github.com/shroudb/shroudb-keep-go"
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
	uri := os.Getenv("SHROUDB_KEEP_TEST_URI")
	if uri == "" {
		uri = "shroudb-keep://127.0.0.1:6799"
	}

	client, err := shroudb_keep.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health
	err = client.Health("")
	check("health", err == nil)

	// 2. PUT db/test/secret-go
	value := base64.StdEncoding.EncodeToString([]byte("my-secret-value"))
	_, err = client.Put("db/test/secret-go", value, nil)
	check("put", err == nil)

	// 3. GET db/test/secret-go
	result, err := client.Get("db/test/secret-go", nil)
	check("get", err == nil && result != nil)

	// 4. PUT db/test/secret-go (version 2)
	value2 := base64.StdEncoding.EncodeToString([]byte("my-updated-secret"))
	_, err = client.Put("db/test/secret-go", value2, nil)
	check("put_v2", err == nil)

	// 5. GET db/test/secret-go VERSION 1
	resultV1, err := client.Get("db/test/secret-go", &shroudb_keep.GetOptions{Version: "1"})
	check("get_v1", err == nil && resultV1 != nil)

	// 6. VERSIONS db/test/secret-go
	_, err = client.Versions("db/test/secret-go")
	check("versions", err == nil)

	// 7. LIST db/
	_, err = client.List("db/")
	check("list", err == nil)

	// 8. DELETE db/test/secret-go
	_, err = client.Delete("db/test/secret-go")
	check("delete", err == nil)

	// 9. Error: GET db/test/secret-go (deleted)
	_, err = client.Get("db/test/secret-go", nil)
	check("error_deleted", err != nil)

	// 10. Error: GET nonexistent/path
	_, err = client.Get("nonexistent/path", nil)
	check("error_notfound", err != nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
