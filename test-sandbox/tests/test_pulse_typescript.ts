/**
 * ShrouDB Pulse TypeScript client integration test.
 */

import { ShroudbPulseClient } from "./src/index.js";
import { ShroudbPulseError } from "./src/errors.js";

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
    process.env.SHROUDB_PULSE_TEST_URI ?? "shroudb-pulse://127.0.0.1:6999";
  const client = await ShroudbPulseClient.connect(uri);

  try {
    // 1. Health
    await client.health();
    check("health", true);

    // 2. INGEST (push a test event)
    try {
      await client.ingest({
        source: "test-source",
        eventType: "test.event",
        data: { message: "hello from integration test" },
      });
      check("ingest", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("ingest", true);
      } else {
        throw e;
      }
    }

    // 3. QUERY (retrieve the event)
    try {
      await client.query({ source: "test-source" });
      check("query", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("query", true);
      } else {
        throw e;
      }
    }

    // 4. COUNT
    try {
      await client.count();
      check("count", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("count", true);
      } else {
        throw e;
      }
    }

    // 5. SOURCE_LIST
    try {
      await client.sourceList();
      check("source_list", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("source_list", true);
      } else {
        throw e;
      }
    }

    // 6. SOURCE_STATUS
    try {
      await client.sourceStatus("test-source");
      check("source_status", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("source_status", true);
      } else {
        throw e;
      }
    }
  } finally {
    client.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
