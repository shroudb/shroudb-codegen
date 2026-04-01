"""ShrouDB Courier unified SDK integration test."""

import asyncio
import json
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
    uri = os.environ.get("SHROUDB_COURIER_TEST_URI", "shroudb-courier://127.0.0.1:6799")
    db = ShrouDB(courier=uri)

    try:
        # health
        try:
            result = await db.courier.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # channel_list
        try:
            result = await db.courier.channel_list()
            check("channel_list", result is not None)
        except Exception as e:
            check("channel_list", False)
            print(f"    error: {e}")

        # channel_create
        import time
        channel_name = f"test-channel-{int(time.time()) % 10000}"
        try:
            config = json.dumps({"url": "https://example.com/webhook"})
            result = await db.courier.channel_create(channel_name, "webhook", config)
            check("channel_create", result is not None and result.name == channel_name)
        except ShrouDBError as e:
            ok = "EXISTS" in str(e) or "exists" in str(e).lower()
            check("channel_create", ok)
            if not ok:
                print(f"    error: {e}")
        except Exception as e:
            check("channel_create", False)
            print(f"    error: {e}")

        # channel_delete
        try:
            result = await db.courier.channel_delete(channel_name)
            check("channel_delete", result is not None)
        except Exception as e:
            check("channel_delete", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
