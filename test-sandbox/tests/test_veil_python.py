"""ShrouDB Veil unified SDK integration test."""

import asyncio
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
    global passed, failed
    uri = os.environ.get("SHROUDB_VEIL_TEST_URI", "shroudb-veil://127.0.0.1:6999")
    db = ShrouDB(veil=uri)

    try:
        # health
        try:
            result = await db.veil.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # index_create (use unique name per run to avoid "already exists")
        import time
        idx_name = f"test-idx-{int(time.time()) % 10000}"
        try:
            result = await db.veil.index_create(idx_name)
            check("index_create", result is not None)
        except ShrouDBError as e:
            # EXISTS is ok if index was created in a previous run
            check("index_create", "EXISTS" in str(e) or "exists" in str(e))
        except Exception as e:
            check("index_create", False)
            print(f"    error: {e}")

        # tokenize (veil expects base64-encoded plaintext)
        import base64
        plaintext_b64 = base64.b64encode(b"hello").decode()
        try:
            result = await db.veil.tokenize(idx_name, plaintext_b64)
            token = getattr(result, "token", None) or getattr(result, "tokens", None)
            check("tokenize", token is not None)
        except Exception as e:
            check("tokenize", False)
            print(f"    error: {e}")

        # put (store blind tokens for an entry)
        try:
            result = await db.veil.put(idx_name, "entry-1", plaintext_b64)
            check("put", result is not None)
        except Exception as e:
            check("put", False)
            print(f"    error: {e}")

        # search (search by token)
        try:
            result = await db.veil.search(idx_name, plaintext_b64)
            check("search", result is not None)
        except Exception as e:
            check("search", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
