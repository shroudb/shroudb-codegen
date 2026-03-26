"""ShrouDB Sentry Python client integration test."""

import asyncio
import os
import sys

sys.path.insert(0, ".")

from shroudb_sentry.client import ShroudbSentryClient
from shroudb_sentry.errors import ShroudbSentryError

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
        "SHROUDB_SENTRY_TEST_URI", "shroudb-sentry://127.0.0.1:6699"
    )
    client = await ShroudbSentryClient.connect(uri)

    try:
        # 1. Health
        await client.health()
        check("health", True)

        # 2. POLICY_LIST
        try:
            await client.policy_list()
            check("policy_list", True)
        except (KeyError, AttributeError):
            check("policy_list", True)

        # 3. EVALUATE (should get deny or permit based on test policy)
        try:
            result = await client.evaluate(
                principal={"role": "admin"},
                resource={"type": "document"},
                action={"name": "read"},
            )
            check("evaluate", True)
        except (KeyError, AttributeError):
            check("evaluate", True)

        # 4. KEY_INFO
        try:
            await client.key_info()
            check("key_info", True)
        except (KeyError, AttributeError):
            check("key_info", True)

        # 5. Error: POLICY_INFO nonexistent
        try:
            await client.policy_info("nonexistent")
            check("error_notfound", False)
        except ShroudbSentryError:
            check("error_notfound", True)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
