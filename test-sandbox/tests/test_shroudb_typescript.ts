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

    // 7. PIPELINE: atomic batch of commands on one round-trip.
    try {
      const results = await db.shroudb.pipeline([
        ["PUT", "test-ns", "pipe-k1", "v1"],
        ["PUT", "test-ns", "pipe-k2", "v2"],
        ["GET", "test-ns", "pipe-k1"],
      ]);
      check("pipeline_returns_array", Array.isArray(results));
      check("pipeline_length", results.length === 3);
      check("pipeline_get_value", (results[2] as { value?: string }).value === "v1");
    } catch (e: unknown) {
      check("pipeline", false);
      console.log(`    error: ${e}`);
    }

    // 8. PIPELINE idempotency: same request_id returns cached result.
    try {
      const rid = `test-idempotency-${Date.now()}`;
      const first = await db.shroudb.pipeline(
        [["PUT", "test-ns", "pipe-idem", "first"]],
        rid,
      );
      const second = await db.shroudb.pipeline(
        [["PUT", "test-ns", "pipe-idem", "second"]],
        rid,
      );
      const firstVersion = (first[0] as { version?: number }).version;
      const secondVersion = (second[0] as { version?: number }).version;
      check("pipeline_idempotent_replay", firstVersion === secondVersion);
      const current = await db.shroudb.get("test-ns", "pipe-idem");
      check(
        "pipeline_idempotent_value_unchanged",
        (current as { value?: string }).value === "first",
      );
    } catch (e: unknown) {
      check("pipeline_idempotency", false);
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
