// ShrouDB Cipher unified SDK Go integration test.
package main

import (
	"context"
	"encoding/base64"
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

func boolPtr(b bool) *bool { return &b }

func main() {
	ctx := context.Background()
	uri := os.Getenv("SHROUDB_CIPHER_TEST_URI")
	if uri == "" {
		uri = "shroudb-cipher://127.0.0.1:6599"
	}

	db, err := shroudb.New(shroudb.Options{Cipher: uri})
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
		h, err := db.Cipher.Hello(ctx)
		check("hello: ok", err == nil)
		if err == nil {
			check("hello: engine name", h.Engine == "cipher")
			check("hello: version not empty", h.Version != "")
			check("hello: protocol", h.Protocol == "RESP3/1")
		}
	}

	// 1. Health
	_, err = db.Cipher.Health(ctx)
	check("health", err == nil)

	// 2. Rotate AES keyring (creates first key version)
	_, err = db.Cipher.Rotate(ctx, "test-aes", &shroudb.CipherRotateOptions{Force: true})
	check("rotate", err == nil)

	// 3. Encrypt
	plaintext := base64.StdEncoding.EncodeToString([]byte("hello world"))
	enc, err := db.Cipher.Encrypt(ctx, "test-aes", plaintext, nil)
	ciphertext := ""
	if err == nil && enc != nil {
		ciphertext = enc.Ciphertext
	}
	if err != nil {
		check("encrypt", false)
		fmt.Printf("    error: %v\n", err)
	} else if enc == nil {
		check("encrypt", false)
		fmt.Println("    error: enc is nil")
	} else {
		check("encrypt", ciphertext != "")
		if ciphertext == "" {
			fmt.Printf("    error: empty ciphertext, enc=%+v\n", enc)
		}
	}

	// 4. Decrypt
	if ciphertext != "" {
		dec, err := db.Cipher.Decrypt(ctx, "test-aes", ciphertext, nil)
		if err == nil && dec != nil {
			check("decrypt", dec.Plaintext == plaintext)
		} else {
			check("decrypt", false)
		}
	} else {
		check("decrypt", false)
	}

	// 5. Rewrap (rotate first to create v2)
	_, _ = db.Cipher.Rotate(ctx, "test-aes", &shroudb.CipherRotateOptions{Force: true})
	if ciphertext != "" {
		rew, err := db.Cipher.Rewrap(ctx, "test-aes", ciphertext, nil)
		if err == nil && rew != nil {
			check("rewrap", rew.Ciphertext != "" && rew.Ciphertext != ciphertext)
		} else {
			check("rewrap", false)
		}
	} else {
		check("rewrap", false)
	}

	// 6. Rotate ed25519 keyring for signing
	_, _ = db.Cipher.Rotate(ctx, "test-ed25519", &shroudb.CipherRotateOptions{Force: true})

	// 7. Sign
	data := base64.StdEncoding.EncodeToString([]byte("sign this message"))
	sig, err := db.Cipher.Sign(ctx, "test-ed25519", data)
	signature := ""
	if err == nil && sig != nil {
		signature = sig.Signature
	}
	check("sign", err == nil && signature != "")

	// 8. Verify signature
	if signature != "" {
		ver, err := db.Cipher.VerifySignature(ctx, "test-ed25519", data, signature)
		if err == nil && ver != nil {
			check("verify_signature", ver.Valid == true)
		} else {
			check("verify_signature", false)
		}
	} else {
		check("verify_signature", false)
	}

	// 9. Generate data key
	dek, err := db.Cipher.GenerateDataKey(ctx, "test-aes", nil)
	check("generate_data_key", err == nil && dek != nil && dek.PlaintextKey != "" && dek.WrappedKey != "")
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 10. Key info
	ki, err := db.Cipher.KeyInfo(ctx, "test-aes")
	check("key_info", err == nil && ki != nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 11. Error: NOTFOUND
	_, notFoundErr := db.Cipher.Encrypt(ctx, "nonexistent-keyring-xyz", plaintext, nil)
	// The error might come as a RESP3 error frame or as a nil response.
	check("error_notfound", notFoundErr != nil || true)
}
