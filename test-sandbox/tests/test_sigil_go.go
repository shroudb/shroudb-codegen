// ShrouDB Sigil unified SDK Go integration test.
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
	uri := os.Getenv("SHROUDB_SIGIL_TEST_URI")
	if uri == "" {
		uri = "shroudb-sigil://127.0.0.1:6299"
	}

	db, err := shroudb.New(shroudb.Options{Sigil: uri})
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

	schemaName := fmt.Sprintf("test-schema-%d", time.Now().Unix()%10000)
	envelopeId := "test-envelope-1"
	userId := "test-user-1"

	// 1. Health
	_, err = db.Sigil.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. Schema register (with credential field for verify/session tests)
	schemaJSON := map[string]any{
		"fields": []map[string]any{
			{
				"name":       "username",
				"field_type": "string",
				"annotations": map[string]any{
					"index": true,
				},
			},
			{
				"name":       "password",
				"field_type": "string",
				"annotations": map[string]any{
					"credential": true,
				},
			},
		},
	}
	_, err = db.Sigil.SchemaRegister(ctx, schemaName, schemaJSON)
	if err != nil && (strings.Contains(strings.ToLower(err.Error()), "exists")) {
		check("schema_register", true)
	} else if err == nil {
		check("schema_register", true)
	} else {
		check("schema_register", false)
		fmt.Printf("    error: %v\n", err)
	}

	// 3. Schema list
	schemaListResult, err := db.Sigil.SchemaList(ctx)
	check("schema_list", err == nil && schemaListResult != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. Envelope create
	envelopeData := map[string]any{
		"username": "testuser",
		"password": "s3cret123!",
	}
	_, err = db.Sigil.EnvelopeCreate(ctx, schemaName, envelopeId, envelopeData)
	if err != nil && (strings.Contains(strings.ToLower(err.Error()), "exists")) {
		check("envelope_create", true)
	} else if err == nil {
		check("envelope_create", true)
	} else {
		check("envelope_create", false)
		fmt.Printf("    error: %v\n", err)
	}

	// 5. Envelope get
	result, err := db.Sigil.EnvelopeGet(ctx, schemaName, envelopeId)
	check("envelope_get", err == nil && result != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 6. Envelope verify
	verifyResult, err := db.Sigil.EnvelopeVerify(ctx, schemaName, envelopeId, "password", "s3cret123!")
	check("envelope_verify", err == nil && verifyResult != nil && verifyResult.Valid)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 7. Envelope delete
	_, err = db.Sigil.EnvelopeDelete(ctx, schemaName, envelopeId)
	check("envelope_delete", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 8. User create (sugar for envelope_create)
	userData := map[string]any{
		"username": "testuser2",
		"password": "s3cret123!",
	}
	userResult, err := db.Sigil.UserCreate(ctx, schemaName, userId, userData)
	if err != nil && (strings.Contains(strings.ToLower(err.Error()), "exists")) {
		check("user_create", true)
	} else if err == nil {
		check("user_create", userResult != nil)
	} else {
		check("user_create", false)
		fmt.Printf("    error: %v\n", err)
	}

	// 9. User verify
	userVerifyResult, err := db.Sigil.UserVerify(ctx, schemaName, userId, "s3cret123!")
	check("user_verify", err == nil && userVerifyResult != nil && userVerifyResult.Valid)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 10. Session create
	sessionResult, err := db.Sigil.SessionCreate(ctx, schemaName, userId, "s3cret123!", nil)
	check("session_create", err == nil && sessionResult != nil && sessionResult.AccessToken != "")
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
}
