/**
 * ShrouDB unified SDK — Sigil engine integration test.
 *
 * Tests schema registration, envelope creation, and envelope retrieval.
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
    process.env.SHROUDB_SIGIL_TEST_URI ?? "shroudb-sigil://127.0.0.1:6299";
  const db = new ShrouDB({ sigil: uri });

  const schemaName = `test-schema-${Math.floor(Date.now() % 10000)}`;
  const envelopeId = "test-envelope-1";
  const userId = "test-user-1";

  try {
    // 1. Health
    try {
      await db.sigil.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 1b. Ping — added in Sigil v2.1 to restore uniform meta-command coverage.
    try {
      await db.sigil.ping();
      check("ping", true);
    } catch (e: unknown) {
      check("ping", false);
      console.log(`    error: ${e}`);
    }

    // 2. Schema register (with credential field for verify/session tests)
    try {
      const schema = {
        fields: [
          { name: "username", field_type: "string", annotations: { index: true } },
          { name: "password", field_type: "string", annotations: { credential: true } },
        ],
      };
      const result = await db.sigil.schemaRegister(schemaName, schema);
      check("schema_register", result != null);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("schema_register", true);
      } else {
        check("schema_register", false);
        console.log(`    error: ${e}`);
      }
    }

    // 3. Schema list
    try {
      const result = await db.sigil.schemaList();
      check("schema_list", result != null);
    } catch (e: unknown) {
      check("schema_list", false);
      console.log(`    error: ${e}`);
    }

    // 4. Envelope create
    try {
      const result = await db.sigil.envelopeCreate(schemaName, envelopeId, {
        username: "testuser",
        password: "s3cret123!",
      });
      check("envelope_create", result != null);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("envelope_create", true);
      } else {
        check("envelope_create", false);
        console.log(`    error: ${e}`);
      }
    }

    // 5. Envelope get
    try {
      const result = await db.sigil.envelopeGet(schemaName, envelopeId);
      check("envelope_get", result != null);
    } catch (e: unknown) {
      check("envelope_get", false);
      console.log(`    error: ${e}`);
    }

    // 6. Envelope verify
    try {
      const result = await db.sigil.envelopeVerify(schemaName, envelopeId, "password", "s3cret123!");
      check("envelope_verify", result?.valid === true);
    } catch (e: unknown) {
      check("envelope_verify", false);
      console.log(`    error: ${e}`);
    }

    // 7. Envelope delete
    try {
      const result = await db.sigil.envelopeDelete(schemaName, envelopeId);
      check("envelope_delete", result != null);
    } catch (e: unknown) {
      check("envelope_delete", false);
      console.log(`    error: ${e}`);
    }

    // 8. User create (sugar for envelope_create)
    try {
      const result = await db.sigil.userCreate(schemaName, userId, {
        username: "testuser2",
        password: "s3cret123!",
      });
      check("user_create", result != null);
    } catch (e: unknown) {
      if (e instanceof ShrouDBError && (String(e).includes("EXISTS") || String(e).toLowerCase().includes("exists"))) {
        check("user_create", true);
      } else {
        check("user_create", false);
        console.log(`    error: ${e}`);
      }
    }

    // 9. User verify
    try {
      const result = await db.sigil.userVerify(schemaName, userId, "s3cret123!");
      check("user_verify", result?.valid === true);
    } catch (e: unknown) {
      check("user_verify", false);
      console.log(`    error: ${e}`);
    }

    // 10. Session create
    try {
      const result = await db.sigil.sessionCreate(schemaName, userId, "s3cret123!");
      check("session_create", result != null && result.access_token != null && result.access_token.length > 0);
    } catch (e: unknown) {
      check("session_create", false);
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
