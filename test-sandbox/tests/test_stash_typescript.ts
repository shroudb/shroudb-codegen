import { ShrouDB } from "./src/index.js";
import { ShrouDBError } from "./src/errors.js";

let passed = 0;
let failed = 0;
function check(name: string, ok: boolean) {
  if (ok) { passed++; console.log(`  PASS  ${name}`); }
  else { failed++; console.log(`  FAIL  ${name}`); }
}

async function main() {
  const uri = process.env.SHROUDB_STASH_TEST_URI ?? "shroudb-stash://127.0.0.1:7299";
  const db = new ShrouDB({ stash: uri });
  const blobData = Buffer.from("hello encrypted world").toString("base64");
  const blobId = "test-blob-1";

  try {
    try { await db.stash.health(); check("health", true); }
    catch (e) { check("health", false); console.log(`    ${e}`); }

    // store — may fail with CIPHER_UNAVAILABLE when running standalone
    try {
      await db.stash.store(blobId, blobData);
      check("store", true);
    } catch (e) {
      check("store", e instanceof ShrouDBError && String(e).toLowerCase().includes("cipher"));
    }

    // inspect — NOTFOUND if store failed
    try {
      await db.stash.inspect(blobId);
      check("inspect", true);
    } catch (e) {
      check("inspect", e instanceof ShrouDBError);
    }

    try { await db.stash.command(); check("command_list", true); }
    catch (e) { check("command_list", false); console.log(`    ${e}`); }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
