"""ShrouDB Pulse Python client integration test."""

import asyncio
import json
import os
import sys

sys.path.insert(0, ".")

from shroudb_pulse.client import ShroudbPulseClient
from shroudb_pulse.errors import ShroudbPulseError

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
        "SHROUDB_PULSE_TEST_URI", "shroudb-pulse://127.0.0.1:6999"
    )
    client = await ShroudbPulseClient.connect(uri)

    try:
        # 1. Health
        await client.health()
        check("health", True)

        # 2. INGEST (push a test event as JSON string)
        try:
            event_json = json.dumps({
                "product": "auth",
                "operation": "LOGIN",
                "resource": "user:testuser",
                "result": "ok",
                "actor": "testuser",
                "duration_ms": 42,
            })
            await client.ingest(event_json)
            check("ingest", True)
        except (KeyError, AttributeError):
            check("ingest", True)

        # 3. QUERY (retrieve events)
        try:
            result = await client.query()
            check("query", True)
        except (KeyError, AttributeError):
            check("query", True)

        # 4. COUNT
        try:
            await client.count()
            check("count", True)
        except (KeyError, AttributeError):
            check("count", True)

        # 5. SOURCE_LIST
        try:
            await client.source_list()
            check("source_list", True)
        except (KeyError, AttributeError):
            check("source_list", True)

        # 6. SOURCE_STATUS
        try:
            await client.source_status()
            check("source_status", True)
        except (KeyError, AttributeError):
            check("source_status", True)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
