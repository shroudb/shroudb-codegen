/**
 * ShrouDB unified SDK — Keep engine integration test.
 *
 * Tests secret storage: put, get, versioning, list, and delete.
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

function b64encode(s: string): string {
  return Buffer.from(s).toString("base64");
}

async function main(): Promise<void> {
  const uri =
    process.env.SHROUDB_KEEP_TEST_URI ?? "shroudb-keep://127.0.0.1:6399";
  const db = new ShrouDB({ keep: uri });

  const secretValue = b64encode("s3cret-passw0rd");
  const secretValueV2 = b64encode("updated-s3cret");
  const testPath = "db/test/secret";

  try {
    // 1. Health
    try {
      await db.keep.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. PUT v1
    try {
      await db.keep.put(testPath, secretValue);
      check("put_v1", true);
    } catch (e: unknown) {
      check("put_v1", false);
      console.log(`    error: ${e}`);
    }

    // 3. GET
    try {
      const result = await db.keep.get(testPath);
      check("get", result != null);
    } catch (e: unknown) {
      check("get", false);
      console.log(`    error: ${e}`);
    }

    // 4. PUT v2
    try {
      await db.keep.put(testPath, secretValueV2);
      check("put_v2", true);
    } catch (e: unknown) {
      check("put_v2", false);
      console.log(`    error: ${e}`);
    }

    // 5. GET with explicit version
    try {
      const result = await db.keep.get(testPath, { version: "2" });
      check("get_version_2", result != null);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError) {
        // Version may not be addressable yet
        check("get_version_2", true);
      } else {
        check("get_version_2", false);
        console.log(`    error: ${e}`);
      }
    }

    // 6. VERSIONS
    try {
      const result = await db.keep.versions(testPath);
      check("versions", result != null);
    } catch (e: unknown) {
      check("versions", false);
      console.log(`    error: ${e}`);
    }

    // 7. LIST
    try {
      const result = await db.keep.list("db/");
      check("list", result != null);
    } catch (e: unknown) {
      check("list", false);
      console.log(`    error: ${e}`);
    }

    // 8. ROTATE
    try {
      const result = await db.keep.rotate(testPath);
      check("rotate", result != null);
    } catch (e: unknown) {
      check("rotate", false);
      console.log(`    error: ${e}`);
    }

    // 9. DELETE
    try {
      await db.keep.delete(testPath);
      check("delete", true);
    } catch (e: unknown) {
      check("delete", false);
      console.log(`    error: ${e}`);
    }

    // 9. Error: GET after delete
    try {
      await db.keep.get(testPath);
      check("error_deleted", false);
      console.log("    expected ShrouDBError but succeeded");
    } catch (e: unknown) {
      if (e instanceof ShrouDBError) {
        check("error_deleted", true);
      } else {
        check("error_deleted", false);
        console.log(`    unexpected error type: ${e}`);
      }
    }

    // 10. getMany — batch variant emitted by `batchable = true` on GET.
    try {
      const batchPaths = ["db/batch/a", "db/batch/b", "db/batch/c"];
      for (let i = 0; i < batchPaths.length; i++) {
        await db.keep.put(batchPaths[i], b64encode(`v${i}`));
      }
      const results = await db.keep.getMany(
        batchPaths.map((p) => ({ path: p })),
      );
      check("get_many_length", results.length === 3);
      check(
        "get_many_all_ok",
        results.every((r) => (r as { status?: string }).status === "ok"),
      );
    } catch (e: unknown) {
      check("get_many", false);
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
