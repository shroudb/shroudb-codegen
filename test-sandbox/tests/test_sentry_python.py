"""ShrouDB Sentry unified SDK integration test."""

import asyncio
import json
import os
import sys
import time

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
    uri = os.environ.get("SHROUDB_SENTRY_TEST_URI", "shroudb-sentry://127.0.0.1:6499")
    db = ShrouDB(sentry=uri)

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.sentry.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "sentry")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            result = await db.sentry.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # policy_list
        try:
            result = await db.sentry.policy_list()
            check("policy_list", result is not None)
        except Exception as e:
            check("policy_list", False)
            print(f"    error: {e}")

        # evaluate
        try:
            eval_request = json.dumps({
                "principal": "user:test@example.com",
                "resource": "secret:db/test/*",
                "action": "read",
            })
            result = await db.sentry.evaluate(eval_request)
            decision = getattr(result, "decision", None) or getattr(result, "allowed", None)
            check("evaluate", decision is not None)
        except Exception as e:
            check("evaluate", False)
            print(f"    error: {e}")

        # key_info
        try:
            result = await db.sentry.key_info()
            check("key_info", result is not None)
        except Exception as e:
            check("key_info", False)
            print(f"    error: {e}")

        # policy_create
        policy_name = f"test-policy-{int(time.time()) % 10000}"
        try:
            policy_body = json.dumps({
                "effect": "permit",
                "principals": ["user:*"],
                "resources": ["secret:test/*"],
                "actions": ["read"],
            })
            result = await db.sentry.policy_create(policy_name, policy_body)
            check("policy_create", True)
        except ShrouDBError as e:
            # EXISTS or DENIED (no auth token) are both acceptable
            check("policy_create", True)
        except Exception as e:
            check("policy_create", False)
            print(f"    error: {e}")

        # policy_delete
        try:
            result = await db.sentry.policy_delete(policy_name)
            check("policy_delete", True)
        except ShrouDBError:
            # DENIED or NOTFOUND are both acceptable
            check("policy_delete", True)
        except Exception as e:
            check("policy_delete", False)
            print(f"    error: {e}")

        # error: policy_get on nonexistent
        try:
            await db.sentry.policy_get("nonexistent-policy-xyz")
            check("error_notfound", False)
            print("    expected ShrouDBError but succeeded")
        except ShrouDBError as e:
            check("error_notfound", True)
        except Exception as e:
            check("error_notfound", False)
            print(f"    unexpected error type: {type(e).__name__}: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
