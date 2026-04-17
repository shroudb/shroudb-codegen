/**
 * ShrouDB unified SDK — Cipher engine integration test.
 *
 * Tests encryption, decryption, key rotation, rewrap, signing, and verification.
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
    process.env.SHROUDB_CIPHER_TEST_URI ?? "shroudb-cipher://127.0.0.1:6599";
  const db = new ShrouDB({ cipher: uri });

  const plaintextB64 = b64encode("hello world");
  const dataB64 = b64encode("sign this message");

  try {
    // Handshake sanity — every engine must answer HELLO.
    try {
      const h = await db.cipher.hello();
      check("hello: ok", true);
      check("hello: engine name", h.engine === "cipher");
      check("hello: version not empty", typeof h.version === "string" && h.version.length > 0);
      check("hello: protocol", h.protocol === "RESP3/1");
    } catch (e) {
      check("hello: ok", false);
    }

    // 1. Health
    try {
      await db.cipher.health();
      check("health", true);
    } catch (e: unknown) {
      check("health", false);
      console.log(`    error: ${e}`);
    }

    // 2. Rotate AES keyring
    try {
      await db.cipher.rotate("test-aes", { force: true });
      check("rotate_aes", true);
    } catch (e: unknown) {
      check("rotate_aes", false);
      console.log(`    error: ${e}`);
    }

    // 3. Encrypt
    let ciphertext: string | null = null;
    try {
      const result = await db.cipher.encrypt("test-aes", plaintextB64);
      ciphertext = result?.ciphertext ?? null;
      check("encrypt", ciphertext != null && ciphertext.length > 0);
    } catch (e: unknown) {
      check("encrypt", false);
      console.log(`    error: ${e}`);
    }

    // 4. Decrypt
    if (ciphertext) {
      try {
        const result = await db.cipher.decrypt("test-aes", ciphertext);
        check("decrypt", result?.plaintext === plaintextB64);
      } catch (e: unknown) {
        check("decrypt", false);
        console.log(`    error: ${e}`);
      }
    } else {
      check("decrypt", false);
      console.log("    skipped: no ciphertext from encrypt");
    }

    // 5. Rewrap
    if (ciphertext) {
      try {
        const result = await db.cipher.rewrap("test-aes", ciphertext);
        check("rewrap", result?.ciphertext != null && result.ciphertext !== ciphertext);
      } catch (e: unknown) {
        check("rewrap", false);
        console.log(`    error: ${e}`);
      }
    } else {
      check("rewrap", false);
      console.log("    skipped: no ciphertext from encrypt");
    }

    // 6. Rotate ed25519 keyring
    try {
      await db.cipher.rotate("test-ed25519", { force: true });
      check("rotate_ed25519", true);
    } catch (e: unknown) {
      check("rotate_ed25519", false);
      console.log(`    error: ${e}`);
    }

    // 7. Sign
    let signature: string | null = null;
    try {
      const result = await db.cipher.sign("test-ed25519", dataB64);
      signature = result?.signature ?? null;
      check("sign", signature != null && signature.length > 0);
    } catch (e: unknown) {
      check("sign", false);
      console.log(`    error: ${e}`);
    }

    // 8. Verify signature
    if (signature) {
      try {
        const result = await db.cipher.verifySignature("test-ed25519", dataB64, signature);
        check("verify_signature", result?.valid === true || (result as any)?.valid === "true");
      } catch (e: unknown) {
        check("verify_signature", false);
        console.log(`    error: ${e}`);
      }
    } else {
      check("verify_signature", false);
      console.log("    skipped: no signature from sign");
    }

    // 9. Generate data key
    try {
      const result = await db.cipher.generateDataKey("test-aes");
      check("generate_data_key", result?.plaintext_key != null && result?.wrapped_key != null);
    } catch (e: unknown) {
      check("generate_data_key", false);
      console.log(`    error: ${e}`);
    }

    // 10. Key info
    try {
      const result = await db.cipher.keyInfo("test-aes");
      check("key_info", result?.keyring === "test-aes");
    } catch (e: unknown) {
      check("key_info", false);
      console.log(`    error: ${e}`);
    }

    // 11. Error: NOTFOUND
    try {
      await db.cipher.encrypt("nonexistent-keyring-xyz", plaintextB64);
      check("error_notfound", false);
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
