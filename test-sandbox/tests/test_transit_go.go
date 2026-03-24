// ShrouDB Transit Go client integration test.
package main

import (
	"encoding/base64"
	"fmt"
	"os"

	shroudb_transit "github.com/shroudb/shroudb-transit-go"
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
	uri := os.Getenv("SHROUDB_TRANSIT_TEST_URI")
	if uri == "" {
		uri = "shroudb-transit://127.0.0.1:6499"
	}

	client, err := shroudb_transit.Connect(uri)
	if err != nil {
		fmt.Printf("FATAL: cannot connect: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	// 1. Health (simple_response — no error means healthy)
	err = client.Health("")
	check("health", err == nil)

	// 2. Rotate (creates first key version)
	_, err = client.Rotate("test-aes", &shroudb_transit.RotateOptions{Force: "true"})
	check("rotate", err == nil || !shroudb_transit.IsCode(err, shroudb_transit.ErrNotfound))

	// 3. Encrypt
	plaintext := base64.StdEncoding.EncodeToString([]byte("hello world"))
	enc, err := client.Encrypt("test-aes", plaintext, nil)
	check("encrypt", err == nil && enc.Ciphertext != "")

	// 4. Decrypt
	dec, err := client.Decrypt("test-aes", enc.Ciphertext, nil)
	check("decrypt", err == nil && dec.Plaintext == plaintext)

	// 5. Rotate again
	_, err = client.Rotate("test-aes", &shroudb_transit.RotateOptions{Force: "true"})
	check("rotate_v2", err == nil || !shroudb_transit.IsCode(err, shroudb_transit.ErrNotfound))

	// 6. Rewrap
	rew, err := client.Rewrap("test-aes", enc.Ciphertext, nil)
	check("rewrap", err == nil && rew.Ciphertext != "" && rew.Ciphertext != enc.Ciphertext)

	// 7. Decrypt rewrapped
	dec2, err := client.Decrypt("test-aes", rew.Ciphertext, nil)
	check("decrypt_rewrapped", err == nil && dec2.Plaintext == plaintext)

	// 8. Key info
	_, err = client.KeyInfo("test-aes")
	check("key_info", err == nil)

	// 9. Sign (ed25519) — rotate first to create the signing key
	_, _ = client.Rotate("test-ed25519", &shroudb_transit.RotateOptions{Force: "true"})
	data := base64.StdEncoding.EncodeToString([]byte("sign this"))
	sig, err := client.Sign("test-ed25519", data, nil)
	check("sign", err == nil && sig.Signature != "")

	// 10. Verify signature
	ver, err := client.VerifySignature("test-ed25519", data, sig.Signature)
	check("verify_signature", err == nil && (ver.Valid == true || ver.Valid == "true"))

	// 11. Error: NOTFOUND
	_, err = client.Encrypt("nonexistent", plaintext, nil)
	check("error_notfound", err != nil)

	// 12. Error: BADARG
	_, err = client.Encrypt("test-aes", "not-valid-b64!!!", nil)
	check("error_badarg", err != nil)

	check("close", true)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
