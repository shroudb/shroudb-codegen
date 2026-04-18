/**
 * ShrouDB unified SDK — Scroll engine integration test.
 *
 * Exercises append → read → create_group → read_group → ack →
 * log_info → group_info → delete_group → delete_log. Scroll is
 * configured with a remote Cipher wired by the run-tests harness,
 * so the full APPEND/READ cycle is encrypted end-to-end.
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
    process.env.SHROUDB_SCROLL_TEST_URI ??
    "shroudb-scroll://127.0.0.1:7200";
  const db = new ShrouDB({ scroll: uri });

  const log = `sandbox-log-ts-${Date.now() % 100000}`;
  const group = "workers";

  try {
    // Handshake sanity — every engine must answer HELLO.
    try {
      const h = await db.scroll.hello();
      check("hello: ok", true);
      check("hello: engine name", h.engine === "scroll");
      check("hello: version not empty", typeof h.version === "string" && h.version.length > 0);
      check("hello: protocol", h.protocol === "RESP3/1");
    } catch (e) {
      check("hello: ok", false);
    }

    // health
    try {
      await db.scroll.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // ping
    try {
      const pong = await db.scroll.ping();
      check("ping", pong !== null && pong !== undefined);
    } catch (e: unknown) {
      check("ping", false);
      console.log(`    error: ${e}`);
    }

    // append (creates the log on first call)
    const firstPayload = Buffer.from("hello scroll").toString("base64");
    try {
      const res = await db.scroll.append(log, firstPayload);
      check("append: first", (res as { offset: number }).offset === 0);
    } catch (e: unknown) {
      check("append: first", false);
      console.log(`    error: ${e}`);
    }

    try {
      const res = await db.scroll.append(log, Buffer.from("second").toString("base64"));
      check("append: second", (res as { offset: number }).offset === 1);
    } catch (e: unknown) {
      check("append: second", false);
      console.log(`    error: ${e}`);
    }

    // read
    try {
      const res = await db.scroll.read(log, 0, 10);
      const entries = (res as { entries: unknown[] }).entries ?? [];
      check("read: count", entries.length === 2);
      // Entries are typed as unknown[] (codegen doesn't emit a typed
      // LogEntry); walk the raw object to verify the payload.
      const first = entries[0] as { payload_b64?: string };
      const decoded = first?.payload_b64
        ? Buffer.from(first.payload_b64, "base64").toString()
        : "";
      check("read: payload roundtrip", decoded === "hello scroll");
    } catch (e: unknown) {
      check("read: count", false);
      console.log(`    error: ${e}`);
    }

    // create_group
    try {
      await db.scroll.createGroup(log, group, "earliest");
      check("create_group", true);
    } catch (e: unknown) {
      check("create_group", false);
      console.log(`    error: ${e}`);
    }

    // read_group
    try {
      const res = await db.scroll.readGroup(log, group, "reader-1", 10);
      const entries = (res as { entries: unknown[] }).entries ?? [];
      check("read_group: count", entries.length === 2);
    } catch (e: unknown) {
      check("read_group: count", false);
      console.log(`    error: ${e}`);
    }

    // ack
    try {
      await db.scroll.ack(log, group, 0);
      await db.scroll.ack(log, group, 1);
      check("ack", true);
    } catch (e: unknown) {
      check("ack", false);
      console.log(`    error: ${e}`);
    }

    // log_info
    try {
      const info = await db.scroll.logInfo(log) as {
        entries_minted: number;
        groups?: string[] | null;
      };
      check("log_info: entries_minted", info.entries_minted === 2);
      check("log_info: has group", (info.groups ?? []).includes(group));
    } catch (e: unknown) {
      check("log_info", false);
      console.log(`    error: ${e}`);
    }

    // group_info
    try {
      const info = await db.scroll.groupInfo(log, group) as {
        last_delivered_offset: number;
        pending_count: number;
      };
      check("group_info: cursor", info.last_delivered_offset === 1);
      check("group_info: pending_count", info.pending_count === 0);
    } catch (e: unknown) {
      check("group_info", false);
      console.log(`    error: ${e}`);
    }

    // delete_group
    try {
      await db.scroll.deleteGroup(log, group);
      check("delete_group", true);
    } catch (e: unknown) {
      check("delete_group", false);
      console.log(`    error: ${e}`);
    }

    // delete_log
    try {
      await db.scroll.deleteLog(log);
      check("delete_log", true);
    } catch (e: unknown) {
      check("delete_log", false);
      console.log(`    error: ${e}`);
    }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
