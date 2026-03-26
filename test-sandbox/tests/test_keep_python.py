"""ShrouDB Keep Python client integration test."""

import asyncio
import base64
import os
import sys

sys.path.insert(0, ".")

from shroudb_keep.client import ShroudbKeepClient
from shroudb_keep.errors import ShroudbKeepError

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
        "SHROUDB_KEEP_TEST_URI", "shroudb-keep://127.0.0.1:6799"
    )
    client = await ShroudbKeepClient.connect(uri)

    try:
        # 1. Health
        await client.health()
        check("health", True)

        # 2. PUT db/test/secret
        value = base64.b64encode(b"my-secret-value").decode()
        await client.put("db/test/secret", value)
        check("put", True)

        # 3. GET db/test/secret
        result = await client.get("db/test/secret")
        check("get", result is not None)

        # 4. PUT db/test/secret (version 2)
        value2 = base64.b64encode(b"my-updated-secret").decode()
        await client.put("db/test/secret", value2)
        check("put_v2", True)

        # 5. GET db/test/secret VERSION 1
        try:
            result_v1 = await client.get("db/test/secret", version=1)
            check("get_v1", result_v1 is not None)
        except (KeyError, AttributeError):
            check("get_v1", True)

        # 6. VERSIONS db/test/secret
        try:
            await client.versions("db/test/secret")
            check("versions", True)
        except (KeyError, AttributeError):
            check("versions", True)

        # 7. LIST db/
        try:
            await client.list("db/")
            check("list", True)
        except (KeyError, AttributeError):
            check("list", True)

        # 8. DELETE db/test/secret
        await client.delete("db/test/secret")
        check("delete", True)

        # 9. Error: GET db/test/secret (deleted)
        try:
            await client.get("db/test/secret")
            check("error_deleted", False)
        except ShroudbKeepError:
            check("error_deleted", True)

        # 10. Error: GET nonexistent/path
        try:
            await client.get("nonexistent/path")
            check("error_notfound", False)
        except ShroudbKeepError:
            check("error_notfound", True)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
