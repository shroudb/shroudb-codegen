/**
 * ShrouDB unified SDK — Forge engine integration test.
 *
 * Tests PKI: CA info, CA listing, and certificate issuance.
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
    process.env.SHROUDB_FORGE_TEST_URI ?? "shroudb-forge://127.0.0.1:6699";
  const db = new ShrouDB({ forge: uri });

  try {
    // Handshake sanity — every engine must answer HELLO.
    try {
      const h = await db.forge.hello();
      check("hello: ok", true);
      check("hello: engine name", h.engine === "forge");
      check("hello: version not empty", typeof h.version === "string" && h.version.length > 0);
      check("hello: protocol", h.protocol === "RESP3/1");
    } catch (e) {
      check("hello: ok", false);
    }

    // 1. Health via ca_list (forge has no RESP3 HEALTH command)
    try {
      await db.forge.caList();
      check("health_via_ca_list", true);
    } catch (e: unknown) {
      check("health_via_ca_list", false);
      console.log(`    error: ${e}`);
    }

    // 2. CA_CREATE — exercises the `SUBJECT` keyword-prefix wire path.
    // Timestamp-suffix the name so parallel / sequential language runs
    // against the same server don't collide on already-created CAs.
    const newCaName = `codegen-new-ca-ts-${Date.now() % 100000}`;
    try {
      const result = await db.forge.caCreate(
        newCaName,
        "ecdsa-p256",
        "CN=Codegen New CA",
        { ttl_days: 30 },
      );
      check("ca_create", result != null && result.name === newCaName);
    } catch (e: unknown) {
      check("ca_create", false);
      console.log(`    error: ${e}`);
    }

    // 3. CA_INFO
    try {
      const result = await db.forge.caInfo("test-ca");
      check("ca_info", result != null);
    } catch (e: unknown) {
      check("ca_info", false);
      console.log(`    error: ${e}`);
    }

    // 3. CA_LIST
    try {
      const result = await db.forge.caList();
      check("ca_list", result != null);
    } catch (e: unknown) {
      check("ca_list", false);
      console.log(`    error: ${e}`);
    }

    // 4. ISSUE certificate
    let serial: string | null = null;
    try {
      const result = await db.forge.issue("test-ca", "CN=test.example.com", "server");
      serial = result?.serial ?? null;
      check("issue", result != null && serial != null);
    } catch (e: unknown) {
      check("issue", false);
      console.log(`    error: ${e}`);
    }

    // 5. INSPECT (use serial from issue)
    if (serial) {
      try {
        const result = await db.forge.inspect("test-ca", serial);
        check("inspect", result != null && result.serial === serial);
      } catch (e: unknown) {
        check("inspect", false);
        console.log(`    error: ${e}`);
      }
    } else {
      check("inspect", false);
      console.log("    skipped: no serial from issue");
    }

    // 6. LIST_CERTS
    try {
      const result = await db.forge.listCerts("test-ca");
      check("list_certs", result != null);
    } catch (e: unknown) {
      check("list_certs", false);
      console.log(`    error: ${e}`);
    }

    // 7. REVOKE (use serial from issue)
    if (serial) {
      try {
        const result = await db.forge.revoke("test-ca", serial);
        check("revoke", result != null);
      } catch (e: unknown) {
        check("revoke", false);
        console.log(`    error: ${e}`);
      }
    } else {
      check("revoke", false);
      console.log("    skipped: no serial from issue");
    }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
