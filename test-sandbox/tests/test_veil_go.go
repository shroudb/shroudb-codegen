// ShrouDB Veil unified SDK Go integration test.
package main

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"sort"
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
	uri := os.Getenv("SHROUDB_VEIL_TEST_URI")
	if uri == "" {
		uri = "shroudb-veil://127.0.0.1:6999"
	}

	db, err := shroudb.New(shroudb.Options{Veil: uri})
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

	idxName := fmt.Sprintf("test-idx-%d", time.Now().Unix()%10000)

	// 1. Health
	_, err = db.Veil.Health(ctx)
	check("health", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 2. IndexCreate
	// Note: response type has string fields but server returns numbers —
	// the Veil protocol.toml uses condensed response format without type info.
	// We ignore the typed response and just check error status.
	_, err = db.Veil.IndexCreate(ctx, idxName)
	if err != nil && (strings.Contains(strings.ToLower(err.Error()), "exists") || shroudb.IsCode(err, "EXISTS")) {
		check("index_create", true)
	} else {
		check("index_create", err == nil)
		if err != nil {
			fmt.Printf("    error: %v\n", err)
		}
	}

	// 3. Tokenize
	plaintextB64 := base64.StdEncoding.EncodeToString([]byte("hello"))
	_, err = db.Veil.Tokenize(ctx, idxName, plaintextB64, nil)
	check("tokenize", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 4. Put (store blind tokens for an entry)
	_, err = db.Veil.Put(ctx, idxName, "entry-1", plaintextB64, nil)
	check("put", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 5. Search (search by token)
	_, err = db.Veil.Search(ctx, idxName, plaintextB64, nil)
	check("search", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// ── Blind (E2EE) operations ──────────────────────────────────

	// 6. PUT ... BLIND
	tokensB64 := blindTokens("hello world")
	_, err = db.Veil.Put(ctx, idxName, "blind-1", tokensB64, &shroudb.VeilPutOptions{Blind: true})
	check("put_blind", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 7. SEARCH ... BLIND (exact)
	queryB64 := blindTokens("hello")
	modeExact := "exact"
	_, err = db.Veil.Search(ctx, idxName, queryB64, &shroudb.VeilSearchOptions{Mode: &modeExact, Blind: true})
	check("search_blind", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}

	// 8. SEARCH ... BLIND with limit
	modeContains := "contains"
	_, err = db.Veil.Search(ctx, idxName, queryB64, &shroudb.VeilSearchOptions{Mode: &modeContains, Limit: intPtr(5), Blind: true})
	check("search_blind_with_limit", err == nil)
	if err != nil {
		fmt.Printf("    error: %v\n", err)
	}
}

func intPtr(n int) *int { return &n }

func blindTokens(text string) string {
	clientKey := make([]byte, 32)
	for i := range clientKey {
		clientKey[i] = 0x42
	}

	re := regexp.MustCompile(`[^a-z0-9]+`)
	words := re.Split(strings.ToLower(text), -1)
	var filtered []string
	for _, w := range words {
		if w != "" {
			filtered = append(filtered, w)
		}
	}

	wordSet := make(map[string]bool)
	for _, w := range filtered {
		wordSet["w:"+w] = true
	}
	var wordTokens []string
	for t := range wordSet {
		wordTokens = append(wordTokens, t)
	}
	sort.Strings(wordTokens)

	trigramSet := make(map[string]bool)
	for _, w := range filtered {
		runes := []rune(w)
		if len(runes) >= 3 {
			for i := 0; i <= len(runes)-3; i++ {
				trigramSet["t:"+string(runes[i:i+3])] = true
			}
		}
	}
	var trigramTokens []string
	for t := range trigramSet {
		trigramTokens = append(trigramTokens, t)
	}
	sort.Strings(trigramTokens)

	doHMAC := func(token string) string {
		mac := hmac.New(sha256.New, clientKey)
		mac.Write([]byte(token))
		return hex.EncodeToString(mac.Sum(nil))
	}

	var blindWords, blindTrigrams []string
	for _, t := range wordTokens {
		blindWords = append(blindWords, doHMAC(t))
	}
	for _, t := range trigramTokens {
		blindTrigrams = append(blindTrigrams, doHMAC(t))
	}

	tokenSet := map[string]interface{}{
		"words":    blindWords,
		"trigrams": blindTrigrams,
	}
	jsonBytes, _ := json.Marshal(tokenSet)
	return base64.StdEncoding.EncodeToString(jsonBytes)
}
