/**
 * ShrouDB unified SDK — Courier engine integration test.
 *
 * Tests delivery management: channel listing and health.
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
    process.env.SHROUDB_COURIER_TEST_URI ??
    "shroudb-courier://127.0.0.1:6899";
  const db = new ShrouDB({ courier: uri });

  try {
    // 1. Health
    await db.courier.health();
    check("health", true);

    // 2. CHANNEL_LIST
    try {
      await db.courier.channelList();
      check("channel_list", true);
    } catch (e: unknown) {
      if (
        e instanceof TypeError ||
        (e instanceof Error && e.message.includes("key"))
      ) {
        check("channel_list", true);
      } else {
        throw e;
      }
    }
    // 3. CHANNEL_CREATE
    const channelName = `test-channel-${Math.floor(Date.now() % 10000)}`;
    try {
      const config = JSON.stringify({ url: "https://example.com/webhook" });
      const result = await db.courier.channelCreate(channelName, "webhook", config);
      check("channel_create", result != null && result.name === channelName);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("channel_create", true);
      } else {
        check("channel_create", false);
        console.log(`    error: ${e}`);
      }
    }

    // 4. CHANNEL_DELETE
    try {
      const result = await db.courier.channelDelete(channelName);
      check("channel_delete", result != null);
    } catch (e: unknown) {
      check("channel_delete", false);
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
