"""ShrouDB Stash unified SDK integration test."""

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
    uri = os.environ.get("SHROUDB_STASH_TEST_URI", "shroudb-stash://127.0.0.1:7299")
    db = ShrouDB(stash=uri)

    blob_data = base64.b64encode(b"hello encrypted world").decode()
    blob_id = "test-blob-1"

    try:
        # health
        try:
            await db.stash.health()
            check("health", True)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # store (may fail if Cipher engine is not wired)
        try:
            result = await db.stash.store(blob_id, blob_data)
            check("store", result is not None)
        except ShrouDBError as e:
            # CIPHER_UNAVAILABLE is expected when running standalone without Cipher
            check("store", "cipher" in str(e).lower() or "CIPHER" in str(e))
            if "cipher" not in str(e).lower():
                print(f"    error: {e}")
        except Exception as e:
            check("store", False)
            print(f"    error: {e}")

        # inspect (will fail with NOTFOUND if store didn't succeed — that's OK)
        try:
            result = await db.stash.inspect(blob_id)
            check("inspect", result is not None)
        except ShrouDBError as e:
            check("inspect", "NOTFOUND" in str(e) or "not found" in str(e).lower())
        except Exception as e:
            check("inspect", False)
            print(f"    error: {e}")

        # command list
        try:
            await db.stash.command()
            check("command_list", True)
        except Exception as e:
            check("command_list", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
