"""ShrouDB Chronicle unified SDK integration test."""

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
    uri = os.environ.get("SHROUDB_CHRONICLE_TEST_URI", "shroudb-chronicle://127.0.0.1:6899")
    db = ShrouDB(chronicle=uri)

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.chronicle.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "chronicle")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            result = await db.chronicle.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # ingest
        try:
            import time
            event = json.dumps({
                "id": "test-event-1",
                "engine": "shroudb",
                "operation": "sdk_test",
                "resource": "test/resource",
                "result": "ok",
                "actor": "user:test@example.com",
                "timestamp": int(time.time()),
                "duration_ms": 1,
            })
            result = await db.chronicle.ingest(event)
            check("ingest", result is not None)
        except Exception as e:
            check("ingest", False)
            print(f"    error: {e}")

        # query
        try:
            result = await db.chronicle.query()
            check("query", result is not None)
        except Exception as e:
            check("query", False)
            print(f"    error: {e}")

        # count
        try:
            result = await db.chronicle.count()
            count_val = getattr(result, "count", None) or getattr(result, "total", None)
            check("count", count_val is not None)
        except Exception as e:
            check("count", False)
            print(f"    error: {e}")

        # ingest_batch
        try:
            batch = [
                {
                    "id": "batch-event-1",
                    "engine": "shroudb",
                    "operation": "sdk_test_batch",
                    "resource": "test/batch",
                    "result": "ok",
                    "actor": "user:batch@example.com",
                    "timestamp": int(time.time()),
                    "duration_ms": 2,
                },
                {
                    "id": "batch-event-2",
                    "engine": "shroudb",
                    "operation": "sdk_test_batch",
                    "resource": "test/batch",
                    "result": "ok",
                    "actor": "user:batch@example.com",
                    "timestamp": int(time.time()),
                    "duration_ms": 3,
                },
            ]
            result = await db.chronicle.ingest_batch(batch)
            check("ingest_batch", result is not None)
        except Exception as e:
            check("ingest_batch", False)
            print(f"    error: {e}")

        # actors
        try:
            result = await db.chronicle.actors()
            check("actors", result is not None)
        except Exception as e:
            check("actors", False)
            print(f"    error: {e}")

        # hotspots
        try:
            result = await db.chronicle.hotspots()
            check("hotspots", result is not None)
        except Exception as e:
            check("hotspots", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
