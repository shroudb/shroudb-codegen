/**
 * ShrouDB unified SDK — ShrouDB core KV engine integration test.
 *
 * Tests namespace creation, put, get, delete, and error after delete.
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
    process.env.SHROUDB_SHROUDB_TEST_URI ?? "shroudb://127.0.0.1:6399";
  const db = new ShrouDB({ shroudb: uri });

  try {
    // 1. Health
    try {
      await db.shroudb.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. Namespace create (required before PUT/GET in v1)
    try {
      await db.shroudb.namespaceCreate("test-ns");
      check("namespace_create", true);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("namespace_create", true);
      } else {
        check("namespace_create", false);
        console.log(`    error: ${e}`);
      }
    }

    // 3. PUT
    try {
      await db.shroudb.put("test-ns", "test-key", "test-value");
      check("put", true);
    } catch (e: unknown) {
      check("put", false);
      console.log(`    error: ${e}`);
    }

    // 4. GET
    try {
      const result = await db.shroudb.get("test-ns", "test-key");
      check("get", result != null);
    } catch (e: unknown) {
      check("get", false);
      console.log(`    error: ${e}`);
    }

    // 5. DELETE
    try {
      await db.shroudb.delete("test-ns", "test-key");
      check("delete", true);
    } catch (e: unknown) {
      check("delete", false);
      console.log(`    error: ${e}`);
    }

    // 6. Error: GET after delete
    try {
      await db.shroudb.get("test-ns", "test-key");
      check("error_after_delete", false);
      console.log("    expected ShrouDBError but succeeded");
    } catch (e: unknown) {
      if (e instanceof ShrouDBError) {
        check("error_after_delete", true);
      } else {
        check("error_after_delete", false);
        console.log(`    unexpected error type: ${e}`);
      }
    }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
