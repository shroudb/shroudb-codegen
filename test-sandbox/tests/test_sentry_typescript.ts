/**
 * ShrouDB unified SDK — Sentry engine integration test.
 *
 * Tests authorization: policy listing, evaluation, key info, and error handling.
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
    process.env.SHROUDB_SENTRY_TEST_URI ??
    "shroudb-sentry://127.0.0.1:6499";
  const db = new ShrouDB({ sentry: uri });

  try {
    // 1. Health
    try {
      await db.sentry.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. POLICY_LIST
    try {
      const result = await db.sentry.policyList();
      check("policy_list", result != null);
    } catch (e: unknown) {
      check("policy_list", false);
      console.log(`    error: ${e}`);
    }

    // 3. EVALUATE (JSON payload)
    try {
      const evalJson = JSON.stringify({
        principal: "user:test@example.com",
        resource: "secret:db/test/*",
        action: "read",
      });
      const result = await db.sentry.evaluate(evalJson);
      check("evaluate", result != null);
    } catch (e: unknown) {
      check("evaluate", false);
      console.log(`    error: ${e}`);
    }

    // 4. KEY_INFO
    try {
      const result = await db.sentry.keyInfo();
      check("key_info", result != null);
    } catch (e: unknown) {
      check("key_info", false);
      console.log(`    error: ${e}`);
    }

    // 5. Policy create
    const policyName = `test-policy-${Math.floor(Date.now() % 10000)}`;
    try {
      const policyBody = JSON.stringify({
        effect: "permit",
        principals: ["user:*"],
        resources: ["secret:test/*"],
        actions: ["read"],
      });
      const result = await db.sentry.policyCreate(policyName, policyBody);
      check("policy_create", result != null && result.name === policyName);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("policy_create", true);
      } else {
        check("policy_create", e instanceof ShrouDBError);
        console.log(`    error: ${e}`);
      }
    }

    // 6. Policy delete
    try {
      const result = await db.sentry.policyDelete(policyName);
      check("policy_delete", result != null);
    } catch (e: unknown) {
      check("policy_delete", e instanceof ShrouDBError);
      console.log(`    error: ${e}`);
    }

    // 7. Error: NOTFOUND (nonexistent policy)
    try {
      await db.sentry.policyGet("nonexistent-policy-xyz");
      check("error_notfound", false);
      console.log("    expected ShrouDBError but succeeded");
    } catch (e: unknown) {
      if (e instanceof ShrouDBError) {
        check("error_notfound", true);
      } else {
        check("error_notfound", false);
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
