/**
 * ShrouDB unified SDK — Veil engine integration test.
 *
 * Tests blind indexing: index creation and tokenization.
 */

import { ShrouDB } from "./src/index.js";
import { ShrouDBError } from "./src/errors.js";

let passed = 0;
let failed = 0;

function check(name: string, condition: boolean): void {
  if (condition) {
    passed++;
    console.log(`  PASS  ${name}`);
  } else {
    failed++;
    console.log(`  FAIL  ${name}`);
  }
}

async function main(): Promise<void> {
  const uri =
    process.env.SHROUDB_VEIL_TEST_URI ?? "shroudb-veil://127.0.0.1:6999";
  const db = new ShrouDB({ veil: uri });

  // Use unique index name per run to avoid "already exists"
  const idxName = `test-idx-${Math.floor(Date.now() % 10000)}`;

  try {
    // Handshake sanity — every engine must answer HELLO.
    try {
      const h = await db.veil.hello();
      check("hello: ok", true);
      check("hello: engine name", h.engine === "veil");
      check("hello: version not empty", typeof h.version === "string" && h.version.length > 0);
      check("hello: protocol", h.protocol === "RESP3/1");
    } catch (e) {
      check("hello: ok", false);
    }

    // 1. Health
    try {
      await db.veil.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. INDEX_CREATE
    try {
      await db.veil.indexCreate(idxName);
      check("index_create", true);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("index_create", true);
      } else {
        check("index_create", false);
        console.log(`    error: ${e}`);
      }
    }

    // 3. TOKENIZE (veil expects base64-encoded plaintext)
    try {
      const plaintextB64 = Buffer.from("hello").toString("base64");
      const result = await db.veil.tokenize(idxName, plaintextB64);
      check("tokenize", result != null);
    } catch (e: unknown) {
      check("tokenize", false);
      console.log(`    error: ${e}`);
    }
    // 4. PUT (store blind tokens for an entry)
    try {
      const plaintextB64 = Buffer.from("hello").toString("base64");
      const result = await db.veil.put(idxName, "entry-1", plaintextB64);
      check("put", result != null);
    } catch (e: unknown) {
      check("put", false);
      console.log(`    error: ${e}`);
    }

    // 5. SEARCH (search by token)
    try {
      const plaintextB64 = Buffer.from("hello").toString("base64");
      const result = await db.veil.search(idxName, plaintextB64);
      check("search", result != null);
    } catch (e: unknown) {
      check("search", false);
      console.log(`    error: ${e}`);
    }

    // ── Blind (E2EE) operations ──────────────────────────────────

    // Build blind tokens client-side using Node crypto (HMAC-SHA256).
    // This mirrors what shroudb-veil-blind does in Rust.
    const crypto = await import("node:crypto");
    const clientKey = Buffer.alloc(32, 0x42); // 32 bytes of 0x42

    function blindTokens(text: string): string {
      const words = text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
      const wordTokens = [...new Set(words.map((w) => `w:${w}`))].sort();
      const trigramTokens: string[] = [];
      for (const w of words) {
        if (w.length >= 3) {
          for (let i = 0; i <= w.length - 3; i++) {
            trigramTokens.push(`t:${w.slice(i, i + 3)}`);
          }
        }
      }
      const uniqueTrigrams = [...new Set(trigramTokens)].sort();

      const hmac = (token: string) =>
        crypto.createHmac("sha256", clientKey).update(token).digest("hex");

      const tokenSet = {
        words: wordTokens.map(hmac),
        trigrams: uniqueTrigrams.map(hmac),
      };
      return Buffer.from(JSON.stringify(tokenSet)).toString("base64");
    }

    // 6. PUT ... BLIND
    try {
      const tokensB64 = blindTokens("hello world");
      const result = await db.veil.put(idxName, "blind-1", tokensB64, { blind: true });
      check("put_blind", result != null);
    } catch (e: unknown) {
      check("put_blind", false);
      console.log(`    error: ${e}`);
    }

    // 7. SEARCH ... BLIND (exact match)
    try {
      const queryB64 = blindTokens("hello");
      const result = await db.veil.search(idxName, queryB64, { mode: "exact", blind: true });
      check("search_blind", result != null && (result as any).matched >= 1);
    } catch (e: unknown) {
      check("search_blind", false);
      console.log(`    error: ${e}`);
    }

    // 8. SEARCH ... BLIND with limit
    try {
      const queryB64 = blindTokens("hello");
      const result = await db.veil.search(idxName, queryB64, { mode: "contains", limit: 5, blind: true });
      check("search_blind_with_limit", result != null);
    } catch (e: unknown) {
      check("search_blind_with_limit", false);
      console.log(`    error: ${e}`);
    }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
