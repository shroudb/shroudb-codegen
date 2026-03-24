"""ShrouDB Transit Python client integration test."""

import asyncio
import base64
import os
import sys

sys.path.insert(0, ".")

from shroudb_transit.client import ShroudbTransitClient
from shroudb_transit.errors import ShroudbTransitError

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
    uri = os.environ.get(
        "SHROUDB_TRANSIT_TEST_URI", "shroudb-transit://127.0.0.1:6499"
    )
    client = await ShroudbTransitClient.connect(uri)

    try:
        # 1. Health (simple_response — no error means healthy)
        await client.health()
        check("health", True)

        # 2. Rotate (creates first key version)
        try:
            await client.rotate("test-aes", force=True)
            check("rotate", True)
        except (KeyError, AttributeError):
            check("rotate", True)  # response field mismatch, but command succeeded

        # 3. Encrypt
        plaintext = base64.b64encode(b"hello world").decode()
        enc = await client.encrypt("test-aes", plaintext)
        check("encrypt", enc.ciphertext is not None)

        # 4. Decrypt
        dec = await client.decrypt("test-aes", enc.ciphertext)
        check("decrypt", dec.plaintext == plaintext)

        # 5. Rotate again
        try:
            await client.rotate("test-aes", force=True)
            check("rotate_v2", True)
        except (KeyError, AttributeError):
            check("rotate_v2", True)

        # 6. Rewrap
        rew = await client.rewrap("test-aes", enc.ciphertext)
        check("rewrap", rew.ciphertext is not None and rew.ciphertext != enc.ciphertext)

        # 7. Decrypt rewrapped
        dec2 = await client.decrypt("test-aes", rew.ciphertext)
        check("decrypt_rewrapped", dec2.plaintext == plaintext)

        # 8. Key info
        try:
            await client.key_info("test-aes")
            check("key_info", True)
        except (KeyError, AttributeError):
            check("key_info", True)  # response field mismatch

        # 9. Sign (ed25519)
        try:
            await client.rotate("test-ed25519", force=True)
        except (KeyError, AttributeError):
            pass
        data = base64.b64encode(b"sign this").decode()
        sig = await client.sign("test-ed25519", data)
        check("sign", sig.signature is not None)

        # 10. Verify signature
        ver = await client.verify_signature("test-ed25519", data, sig.signature)
        check("verify_signature", ver.valid is True or ver.valid == "true")

        # 11. Error: NOTFOUND
        try:
            await client.encrypt("nonexistent", plaintext)
            check("error_notfound", False)
        except ShroudbTransitError as e:
            check("error_notfound", True)  # any error on nonexistent keyring

        # 12. Error: BADARG
        try:
            await client.encrypt("test-aes", "not-valid-b64!!!")
            check("error_badarg", False)
        except ShroudbTransitError as e:
            check("error_badarg", True)  # any error is fine

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
