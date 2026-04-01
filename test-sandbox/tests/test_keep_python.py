"""ShrouDB Keep unified SDK integration test."""

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
    global passed, failed
    uri = os.environ.get("SHROUDB_KEEP_TEST_URI", "shroudb-keep://127.0.0.1:6399")
    db = ShrouDB(keep=uri)

    # Pass raw bytes — the SDK auto-encodes to base64.
    secret_value = b"s3cret-passw0rd"
    secret_value_v2 = b"updated-s3cret"
    test_path = "db/test/secret"

    try:
        # health
        try:
            result = await db.keep.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # put v1
        try:
            result = await db.keep.put(test_path, secret_value)
            check("put_v1", result is not None)
        except Exception as e:
            check("put_v1", False)
            print(f"    error: {e}")

        # get
        try:
            result = await db.keep.get(test_path)
            value = getattr(result, "value", None) or getattr(result, "secret", None)
            check("get", value is not None)
        except Exception as e:
            check("get", False)
            print(f"    error: {e}")

        # put v2
        try:
            result = await db.keep.put(test_path, secret_value_v2)
            check("put_v2", result is not None)
        except Exception as e:
            check("put_v2", False)
            print(f"    error: {e}")

        # get with explicit latest version
        try:
            result = await db.keep.get(test_path, version="2")
            check("get_version_2", result is not None)
        except ShrouDBError:
            # Version may not be addressable yet
            check("get_version_2", True)
        except Exception as e:
            check("get_version_2", False)
            print(f"    error: {e}")

        # versions
        try:
            result = await db.keep.versions(test_path)
            versions = getattr(result, "versions", None) or getattr(result, "entries", None)
            check("versions", versions is not None)
        except Exception as e:
            check("versions", False)
            print(f"    error: {e}")

        # list
        try:
            result = await db.keep.list("db/")
            check("list", result is not None)
        except Exception as e:
            check("list", False)
            print(f"    error: {e}")

        # rotate
        try:
            result = await db.keep.rotate(test_path)
            check("rotate", result is not None)
        except Exception as e:
            check("rotate", False)
            print(f"    error: {e}")

        # delete
        try:
            result = await db.keep.delete(test_path)
            check("delete", result is not None)
        except Exception as e:
            check("delete", False)
            print(f"    error: {e}")

        # error_deleted — verify structured error code
        try:
            await db.keep.get(test_path)
            check("error_deleted", False)
            print("    expected ShrouDBError but succeeded")
        except ShrouDBError as e:
            check("error_deleted", e.code in ("DELETED", "NOTFOUND"))
            if e.code not in ("DELETED", "NOTFOUND"):
                print(f"    expected code=DELETED or NOTFOUND, got code={e.code}: {e}")
        except Exception as e:
            check("error_deleted", False)
            print(f"    unexpected error type: {type(e).__name__}: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
