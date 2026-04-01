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
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
