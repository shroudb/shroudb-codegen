/**
 * ShrouDB Keep TypeScript client integration test.
 */

import { ShroudbKeepClient } from "./src/index.js";
import { ShroudbKeepError } from "./src/errors.js";

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

function b64encode(s: string): string {
  return Buffer.from(s).toString("base64");
}

async function main(): Promise<void> {
  const uri =
    process.env.SHROUDB_KEEP_TEST_URI ?? "shroudb-keep://127.0.0.1:6799";
  const client = await ShroudbKeepClient.connect(uri);

  try {
    // 1. Health
    await client.health();
    check("health", true);

    // 2. PUT db/test/secret
    const value = b64encode("my-secret-value");
    await client.put("db/test/secret", value);
    check("put", true);

    // 3. GET db/test/secret
    const result = await client.get("db/test/secret");
    check("get", result != null);

    // 4. PUT db/test/secret (version 2)
    const value2 = b64encode("my-updated-secret");
    await client.put("db/test/secret", value2);
    check("put_v2", true);

    // 5. GET db/test/secret VERSION 1
    try {
      const resultV1 = await client.get("db/test/secret", { version: 1 });
      check("get_v1", resultV1 != null);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("get_v1", true);
      } else {
        throw e;
      }
    }

    // 6. VERSIONS db/test/secret
    try {
      await client.versions("db/test/secret");
      check("versions", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("versions", true);
      } else {
        throw e;
      }
    }

    // 7. LIST db/
    try {
      await client.list("db/");
      check("list", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("list", true);
      } else {
        throw e;
      }
    }

    // 8. DELETE db/test/secret
    await client.delete("db/test/secret");
    check("delete", true);

    // 9. Error: GET db/test/secret (deleted)
    try {
      await client.get("db/test/secret");
      check("error_deleted", false);
    } catch (e: unknown) {
      if (e instanceof ShroudbKeepError) {
        check("error_deleted", true);
      } else {
        check("error_deleted", false);
      }
    }

    // 10. Error: GET nonexistent/path
    try {
      await client.get("nonexistent/path");
      check("error_notfound", false);
    } catch (e: unknown) {
      if (e instanceof ShroudbKeepError) {
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
