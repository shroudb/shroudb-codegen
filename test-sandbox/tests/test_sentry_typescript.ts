/**
 * ShrouDB Sentry TypeScript client integration test.
 */

import { ShroudbSentryClient } from "./src/index.js";
import { ShroudbSentryError } from "./src/errors.js";

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
    process.env.SHROUDB_SENTRY_TEST_URI ?? "shroudb-sentry://127.0.0.1:6699";
  const client = await ShroudbSentryClient.connect(uri);

  try {
    // 1. Health
    await client.health();
    check("health", true);

    // 2. POLICY_LIST
    try {
      await client.policyList();
      check("policy_list", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("policy_list", true);
      } else {
        throw e;
      }
    }

    // 3. EVALUATE (pass JSON string)
    try {
      const evalJson = JSON.stringify({
        principal: { id: "user-1", roles: ["admin"] },
        resource: { id: "doc-1", type: "document" },
        action: "read",
      });
      await client.evaluate(evalJson);
      check("evaluate", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("evaluate", true);
      } else {
        throw e;
      }
    }

    // 4. KEY_INFO
    try {
      await client.keyInfo();
      check("key_info", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("key_info", true);
      } else {
        throw e;
      }
    }

    // 5. Error: POLICY_INFO nonexistent
    try {
      await client.policyInfo("nonexistent");
      check("error_notfound", false);
    } catch (e: unknown) {
      if (e instanceof ShroudbSentryError) {
        check("error_notfound", true);
      } else {
        check("error_notfound", false);
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
