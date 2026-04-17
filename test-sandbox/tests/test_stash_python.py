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
    import time; blob_id = f"test-blob-py-{int(time.time()) % 100000}"

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.stash.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "stash")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            await db.stash.health()
            check("health", True)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # store
        try:
            result = await db.stash.store(blob_id, blob_data)
            check("store", result is not None)
        except Exception as e:
            check("store", False)
            print(f"    error: {e}")

        # inspect
        try:
            result = await db.stash.inspect(blob_id)
            check("inspect", result is not None)
        except Exception as e:
            check("inspect", False)
            print(f"    error: {e}")

        # retrieve
        try:
            result = await db.stash.retrieve(blob_id)
            check("retrieve", result is not None)
        except Exception as e:
            check("retrieve", False)
            print(f"    error: {e}")

        # revoke (soft)
        try:
            result = await db.stash.revoke(blob_id, soft=True)
            check("revoke_soft", result is not None)
        except Exception as e:
            check("revoke_soft", False)
            print(f"    error: {e}")

        # error: retrieve after soft revoke
        try:
            await db.stash.retrieve(blob_id)
            check("error_after_revoke", False)
            print("    expected ShrouDBError but succeeded")
        except ShrouDBError:
            check("error_after_revoke", True)
        except Exception as e:
            check("error_after_revoke", False)
            print(f"    unexpected: {type(e).__name__}: {e}")

        # store another blob and hard revoke (crypto-shred)
        blob_id2 = f"{blob_id}-shred"
        try:
            await db.stash.store(blob_id2, blob_data)
            result = await db.stash.revoke(blob_id2)
            check("revoke_hard", result is not None)
        except Exception as e:
            check("revoke_hard", False)
            print(f"    error: {e}")

        # error: retrieve after hard revoke
        try:
            await db.stash.retrieve(blob_id2)
            check("error_after_shred", False)
            print("    expected ShrouDBError but succeeded")
        except ShrouDBError:
            check("error_after_shred", True)
        except Exception as e:
            check("error_after_shred", False)
            print(f"    unexpected: {type(e).__name__}: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
