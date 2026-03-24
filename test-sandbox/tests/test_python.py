"""
ShrouDB Python client integration test.

Exercises the generated client against a live ShrouDB server.
Expects SHROUDB_TEST_URI env var (e.g. shroudb://127.0.0.1:6399).
"""

import asyncio
import os
import sys

sys.path.insert(0, ".")

from shroudb.client import ShroudbClient
from shroudb.errors import ShroudbError

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
    uri = os.environ.get("SHROUDB_TEST_URI", "shroudb://127.0.0.1:6399")
    client = await ShroudbClient.connect(uri)

    try:
        # 1. Health (server-level)
        h = await client.health()
        check("health", h.state == "ready")

        # 2. Health (keyspace-level)
        hk = await client.health("test-apikeys")
        check("health_keyspace", hk.count is not None)

        # 3. Issue on test-apikeys
        issued = await client.issue("test-apikeys")
        check("issue", issued.credential_id is not None and issued.token is not None)
        cred_id = issued.credential_id
        token = issued.token

        # 4. Verify the token
        verified = await client.verify("test-apikeys", token)
        check("verify", verified.credential_id == cred_id)

        # 5. Inspect
        info = await client.inspect("test-apikeys", cred_id)
        check("inspect_active", info.state == "active")

        # 6. Update metadata
        await client.update("test-apikeys", cred_id, metadata={"env": "test"})
        check("update", True)  # no error means success

        # 7. Inspect after update
        info2 = await client.inspect("test-apikeys", cred_id)
        check("inspect_meta", info2.meta is not None and info2.meta.get("env") == "test")

        # 8. Suspend
        await client.suspend("test-apikeys", cred_id)
        check("suspend", True)

        # 9. Inspect suspended
        info3 = await client.inspect("test-apikeys", cred_id)
        check("inspect_suspended", info3.state == "suspended")

        # 10. Unsuspend
        await client.unsuspend("test-apikeys", cred_id)
        check("unsuspend", True)

        # 11. Revoke
        await client.revoke("test-apikeys", cred_id)
        check("revoke", True)

        # 12. Verify revoked token should fail
        try:
            await client.verify("test-apikeys", token)
            check("verify_revoked", False)  # should have raised
        except ShroudbError as e:
            check("verify_revoked", e.code in ("STATE_ERROR", "NOTFOUND"))

        # 13. Issue JWT with claims
        jwt_issued = await client.issue(
            "test-jwt", claims={"sub": "user123", "role": "admin"}
        )
        check("issue_jwt", jwt_issued.token is not None)

        # 14. Verify JWT
        jwt_verified = await client.verify("test-jwt", jwt_issued.token)
        check("verify_jwt", jwt_verified.claims is not None)

        # 15. JWKS
        jwks = await client.jwks("test-jwt")
        check("jwks", jwks.jwks is not None and len(jwks.jwks) > 0)

        # 16. KEYS (list credentials)
        keys_result = await client.keys("test-apikeys")
        check("keys", keys_result.cursor is not None)

        # 17. Error: BADARG
        try:
            await client.inspect("test-apikeys", "")
            check("error_badarg", False)
        except ShroudbError as e:
            check("error_badarg", e.code == "BADARG")

        # 18. Error: NOTFOUND
        try:
            await client.inspect("test-apikeys", "nonexistent_credential_id")
            check("error_notfound", False)
        except ShroudbError as e:
            check("error_notfound", e.code == "NOTFOUND")

        # 19. Pipeline
        pipe = client.pipeline()
        pipe.issue("test-apikeys")
        pipe.health()
        results = await pipe.execute()
        check("pipeline", len(results) == 2)

        # 20. Subscribe
        try:
            sub_ok = False

            async def subscribe_test():
                nonlocal sub_ok
                sub = await client.subscribe("*")
                async with sub:
                    # Trigger an event from a second client
                    client2 = await ShroudbClient.connect(uri)
                    try:
                        await client2.issue("test-apikeys")
                    finally:
                        await client2.close()

                    # Read the event
                    async for event in sub:
                        if event.event_type and event.keyspace:
                            sub_ok = True
                        break  # one event is enough

            await asyncio.wait_for(subscribe_test(), timeout=5.0)
            check("subscribe", sub_ok)
        except (asyncio.TimeoutError, Exception) as e:
            check("subscribe", False)
            print(f"         ({type(e).__name__}: {e})")

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
