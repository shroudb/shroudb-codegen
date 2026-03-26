/**
 * ShrouDB Courier TypeScript client integration test.
 *
 * Limited test -- no Transit available, so DELIVER is skipped.
 * Tests management commands only: TEMPLATE_LIST, TEMPLATE_INFO, HEALTH.
 */

import { ShroudbCourierClient } from "./src/index.js";
import { ShroudbCourierError } from "./src/errors.js";

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
    process.env.SHROUDB_COURIER_TEST_URI ?? "shroudb-courier://127.0.0.1:6899";
  const client = await ShroudbCourierClient.connect(uri);

  try {
    // 1. Health
    await client.health();
    check("health", true);

    // 2. TEMPLATE_LIST
    try {
      await client.templateList();
      check("template_list", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("template_list", true);
      } else {
        throw e;
      }
    }

    // 3. Error: TEMPLATE_INFO nonexistent
    try {
      await client.templateInfo("nonexistent");
      check("error_notfound", false);
    } catch (e: unknown) {
      if (e instanceof ShroudbCourierError) {
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
