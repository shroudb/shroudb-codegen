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
  const blobId = `test-blob-ts-${Date.now() % 100000}`;

  try {
    // Handshake sanity — every engine must answer HELLO.
    try {
      const h = await db.stash.hello();
      check("hello: ok", true);
      check("hello: engine name", h.engine === "stash");
      check("hello: version not empty", typeof h.version === "string" && h.version.length > 0);
      check("hello: protocol", h.protocol === "RESP3/1");
    } catch (e) {
      check("hello: ok", false);
    }

    try { await db.stash.health(); check("health", true); }
    catch (e) { check("health", false); console.log(`    ${e}`); }

    try {
      const r = await db.stash.store(blobId, blobData);
      check("store", r != null);
    } catch (e) { check("store", false); console.log(`    ${e}`); }

    try {
      const r = await db.stash.inspect(blobId);
      check("inspect", r != null);
    } catch (e) { check("inspect", false); console.log(`    ${e}`); }

    try {
      const r = await db.stash.retrieve(blobId);
      check("retrieve", r != null);
    } catch (e) { check("retrieve", false); console.log(`    ${e}`); }

    try {
      const r = await db.stash.revoke(blobId, { soft: true });
      check("revoke_soft", r != null);
    } catch (e) { check("revoke_soft", false); console.log(`    ${e}`); }

    try {
      await db.stash.retrieve(blobId);
      check("error_after_revoke", false);
    } catch (e) {
      check("error_after_revoke", e instanceof ShrouDBError);
    }

    // Hard revoke (crypto-shred)
    const blobId2 = `${blobId}-shred`;
    try {
      await db.stash.store(blobId2, blobData);
      const r = await db.stash.revoke(blobId2);
      check("revoke_hard", r != null);
    } catch (e) { check("revoke_hard", false); console.log(`    ${e}`); }

    try {
      await db.stash.retrieve(blobId2);
      check("error_after_shred", false);
    } catch (e) {
      check("error_after_shred", e instanceof ShrouDBError);
    }
  } finally {
    await db.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
