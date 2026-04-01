/**
 * ShrouDB unified SDK — Chronicle engine integration test.
 *
 * Tests audit logging: ingest, query, count, and hotspots.
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
    process.env.SHROUDB_CHRONICLE_TEST_URI ??
    "shroudb-chronicle://127.0.0.1:6899";
  const db = new ShrouDB({ chronicle: uri });

  try {
    // 1. Health
    try {
      await db.chronicle.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. INGEST (push a test event as JSON)
    try {
      const eventJson = JSON.stringify({
        id: "test-event-1",
        engine: "shroudb",
        operation: "sdk_test",
        resource: "test/resource",
        result: "ok",
        actor: "user:test@example.com",
        timestamp: Math.floor(Date.now() / 1000),
        duration_ms: 1,
      });
      await db.chronicle.ingest(eventJson);
      check("ingest", true);
    } catch (e: unknown) {
      check("ingest", false);
      console.log(`    error: ${e}`);
    }

    // 3. QUERY (retrieve events)
    try {
      const result = await db.chronicle.query();
      check("query", result != null);
    } catch (e: unknown) {
      check("query", false);
      console.log(`    error: ${e}`);
    }

    // 4. COUNT
    try {
      const result = await db.chronicle.count();
      check("count", result != null);
    } catch (e: unknown) {
      check("count", false);
      console.log(`    error: ${e}`);
    }

    // 5. INGEST_BATCH
    try {
      const batchPayload = [
          {
            id: "batch-event-1",
            engine: "shroudb",
            operation: "sdk_test_batch",
            resource: "test/batch",
            result: "ok",
            actor: "user:batch@example.com",
            timestamp: Math.floor(Date.now() / 1000),
            duration_ms: 2,
          },
          {
            id: "batch-event-2",
            engine: "shroudb",
            operation: "sdk_test_batch",
            resource: "test/batch",
            result: "ok",
            actor: "user:batch@example.com",
            timestamp: Math.floor(Date.now() / 1000),
            duration_ms: 3,
          },
        ];
      const result = await db.chronicle.ingestBatch(batchPayload);
      check("ingest_batch", result != null);
    } catch (e: unknown) {
      check("ingest_batch", false);
      console.log(`    error: ${e}`);
    }

    // 6. ACTORS
    try {
      const result = await db.chronicle.actors();
      check("actors", result != null);
    } catch (e: unknown) {
      check("actors", false);
      console.log(`    error: ${e}`);
    }

    // 7. HOTSPOTS
    try {
      const result = await db.chronicle.hotspots();
      check("hotspots", result != null);
    } catch (e: unknown) {
      check("hotspots", false);
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
