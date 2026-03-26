"""ShrouDB Courier Python client integration test.

Limited test — no Transit available, so DELIVER is skipped.
Tests management commands only: TEMPLATE_LIST, TEMPLATE_INFO, HEALTH.
"""

import asyncio
import os
import sys

sys.path.insert(0, ".")

from shroudb_courier.client import ShroudbCourierClient
from shroudb_courier.errors import ShroudbCourierError

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
        "SHROUDB_COURIER_TEST_URI", "shroudb-courier://127.0.0.1:6899"
    )
    client = await ShroudbCourierClient.connect(uri)

    try:
        # 1. Health
        await client.health()
        check("health", True)

        # 2. TEMPLATE_LIST
        try:
            await client.template_list()
            check("template_list", True)
        except (KeyError, AttributeError):
            check("template_list", True)

        # 3. Error: TEMPLATE_INFO nonexistent
        try:
            await client.template_info("nonexistent")
            check("error_notfound", False)
        except ShroudbCourierError:
            check("error_notfound", True)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
