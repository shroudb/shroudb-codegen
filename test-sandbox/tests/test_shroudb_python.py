"""ShrouDB core unified SDK integration test."""

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
    uri = os.environ.get("SHROUDB_SHROUDB_TEST_URI", "shroudb://127.0.0.1:6399")
    db = ShrouDB(shroudb=uri)

    try:
        # health
        try:
            result = await db.shroudb.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # namespace_create (required before PUT/GET in v1)
        try:
            await db.shroudb.namespace_create("test-ns")
            check("namespace_create", True)
        except ShrouDBError as e:
            # NAMESPACE_EXISTS is fine if already created
            ok = "EXISTS" in str(e) or "exists" in str(e).lower()
            check("namespace_create", ok)
            if not ok:
                print(f"    error: {e}")
        except Exception as e:
            check("namespace_create", False)
            print(f"    error: {e}")

        # put
        try:
            result = await db.shroudb.put("test-ns", "test-key", "test-value")
            check("put", result is not None)
        except Exception as e:
            check("put", False)
            print(f"    error: {e}")

        # get
        try:
            result = await db.shroudb.get("test-ns", "test-key")
            check("get", result is not None)
        except Exception as e:
            check("get", False)
            print(f"    error: {e}")

        # delete
        try:
            result = await db.shroudb.delete("test-ns", "test-key")
            check("delete", result is not None)
        except Exception as e:
            check("delete", False)
            print(f"    error: {e}")

        # error on get after delete
        try:
            await db.shroudb.get("test-ns", "test-key")
            check("error_after_delete", False)
            print("    expected ShrouDBError but succeeded")
        except ShrouDBError:
            check("error_after_delete", True)
        except Exception as e:
            check("error_after_delete", False)
            print(f"    unexpected error type: {type(e).__name__}: {e}")

        # PIPELINE: atomic batch of commands on one round-trip.
        try:
            results = await db.shroudb.pipeline([
                ["PUT", "test-ns", "pipe-k1", "v1"],
                ["PUT", "test-ns", "pipe-k2", "v2"],
                ["GET", "test-ns", "pipe-k1"],
            ])
            check("pipeline_returns_list", isinstance(results, list))
            check("pipeline_length", len(results) == 3)
            check("pipeline_get_value", results[2].get("value") == "v1")
        except Exception as e:
            check("pipeline", False)
            print(f"    error: {e}")

        # PIPELINE idempotency: same request_id returns cached result.
        try:
            import time

            rid = f"test-idempotency-{int(time.time() * 1000)}"
            first = await db.shroudb.pipeline(
                [["PUT", "test-ns", "pipe-idem", "first"]], request_id=rid
            )
            second = await db.shroudb.pipeline(
                [["PUT", "test-ns", "pipe-idem", "second"]], request_id=rid
            )
            check(
                "pipeline_idempotent_replay",
                first[0].get("version") == second[0].get("version"),
            )
            current = await db.shroudb.get("test-ns", "pipe-idem")
            check(
                "pipeline_idempotent_value_unchanged",
                getattr(current, "value", None) == "first",
            )
        except Exception as e:
            check("pipeline_idempotency", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
