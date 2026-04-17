"""ShrouDB Cipher unified SDK integration test."""

import asyncio
import base64
import os
import sys

sys.path.insert(0, ".")

from shroudb import ShrouDB
from shroudb.errors import ShrouDBError

passed = 0
failed = 0


def check(name, condition):
    global passed, failed
    if condition:
        passed += 1
        print(f"  PASS  {name}")
    else:
        failed += 1
        print(f"  FAIL  {name}")


async def main():
    uri = os.environ.get("SHROUDB_CIPHER_TEST_URI", "shroudb-cipher://127.0.0.1:6599")
    db = ShrouDB(cipher=uri)

    # Pass raw bytes — the SDK auto-encodes to base64.
    plaintext_raw = b"hello world"
    data_raw = b"sign this message"

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.cipher.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "cipher")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            await db.cipher.health()
            check("health", True)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # rotate aes keyring
        try:
            await db.cipher.rotate("test-aes", force=True)
            check("rotate_aes", True)
        except Exception as e:
            check("rotate_aes", False)
            print(f"    error: {e}")

        # encrypt
        ciphertext = None
        try:
            result = await db.cipher.encrypt("test-aes", plaintext_raw)
            ciphertext = result.ciphertext
            check("encrypt", ciphertext is not None and len(ciphertext) > 0)
        except Exception as e:
            check("encrypt", False)
            print(f"    error: {e}")

        # decrypt
        if ciphertext:
            try:
                result = await db.cipher.decrypt("test-aes", ciphertext)
                # Response plaintext is base64-encoded; decode and compare.
                decrypted = base64.b64decode(result.plaintext) if result.plaintext else b""
                check("decrypt", decrypted == plaintext_raw)
            except Exception as e:
                check("decrypt", False)
                print(f"    error: {e}")
        else:
            check("decrypt", False)
            print("    skipped: no ciphertext from encrypt")

        # rewrap
        if ciphertext:
            try:
                result = await db.cipher.rewrap("test-aes", ciphertext)
                check("rewrap", result.ciphertext is not None and result.ciphertext != ciphertext)
            except Exception as e:
                check("rewrap", False)
                print(f"    error: {e}")
        else:
            check("rewrap", False)
            print("    skipped: no ciphertext from encrypt")

        # rotate ed25519 keyring
        try:
            await db.cipher.rotate("test-ed25519", force=True)
            check("rotate_ed25519", True)
        except Exception as e:
            check("rotate_ed25519", False)
            print(f"    error: {e}")

        # sign
        signature = None
        try:
            result = await db.cipher.sign("test-ed25519", data_raw)
            signature = result.signature
            check("sign", signature is not None and len(signature) > 0)
        except Exception as e:
            check("sign", False)
            print(f"    error: {e}")

        # verify_signature
        if signature:
            try:
                result = await db.cipher.verify_signature("test-ed25519", data_raw, signature)
                check("verify_signature", result.valid is True or str(result.valid).lower() == "true")
            except Exception as e:
                check("verify_signature", False)
                print(f"    error: {e}")
        else:
            check("verify_signature", False)
            print("    skipped: no signature from sign")

        # generate_data_key
        try:
            result = await db.cipher.generate_data_key("test-aes")
            check("generate_data_key", result is not None and result.plaintext_key != "" and result.wrapped_key != "")
        except Exception as e:
            check("generate_data_key", False)
            print(f"    error: {e}")

        # key_info
        try:
            result = await db.cipher.key_info("test-aes")
            check("key_info", result is not None and result.keyring == "test-aes")
        except Exception as e:
            check("key_info", False)
            print(f"    error: {e}")

        # error_notfound — verify structured error code is inferred
        try:
            await db.cipher.encrypt("nonexistent-keyring-xyz", plaintext_raw)
            check("error_notfound", False)
        except ShrouDBError as e:
            check("error_notfound", e.code == "NOTFOUND")
            if e.code != "NOTFOUND":
                print(f"    expected code=NOTFOUND, got code={e.code}: {e}")
        except Exception as e:
            check("error_notfound", False)
            print(f"    unexpected error type: {type(e).__name__}: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
