"""ShrouDB Auth HTTP Python client integration test."""

import asyncio
import os
import sys

sys.path.insert(0, ".")

from shroudb_auth.client import ShroudbAuthClient
from shroudb_auth.errors import ShroudbAuthError

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
    base_url = os.environ.get("SHROUDB_AUTH_TEST_URL", "http://127.0.0.1:4001")
    client = ShroudbAuthClient(base_url, keyspace="default")

    try:
        # 1. Health
        h = await client.health()
        check("health", h.status in ("healthy", "ok", "OK"))

        # 2. Signup
        signup = await client.signup(user_id="testuser1", password="testpass123")
        check("signup", signup.access_token is not None and signup.refresh_token is not None)
        access = signup.access_token
        refresh = signup.refresh_token

        # 3. Session (verify access token)
        client._access_token = access
        session = await client.session()
        check("session", session.user_id == "testuser1")

        # 4. Login
        login = await client.login(user_id="testuser1", password="testpass123")
        check("login", login.access_token is not None)

        # 5. Refresh
        client._refresh_token = refresh
        ref = await client.refresh()
        check("refresh", ref.access_token is not None)

        # 6. Change password
        client._access_token = login.access_token
        cp = await client.change_password(
            old_password="testpass123", new_password="newpass456"
        )
        check("change_password", True)

        # 7. Login with new password
        login2 = await client.login(user_id="testuser1", password="newpass456")
        check("login_new_pass", login2.access_token is not None)

        # 8. Forgot password
        fp = await client.forgot_password(user_id="testuser1")
        check("forgot_password", fp.reset_token is not None)

        # 9. Reset password
        rp = await client.reset_password(token=fp.reset_token, new_password="resetpass789")
        check("reset_password", True)

        # 10. Login after reset
        login3 = await client.login(user_id="testuser1", password="resetpass789")
        check("login_after_reset", login3.access_token is not None)

        # 11. Logout
        client._access_token = login3.access_token
        client._refresh_token = login3.refresh_token
        lo = await client.logout()
        check("logout", True)

        # 12. JWKS
        jwks = await client.jwks()
        check("jwks", jwks.keys is not None)

        # 13. Error: wrong password
        try:
            await client.login(user_id="testuser1", password="wrongpass")
            check("error_unauthorized", False)
        except ShroudbAuthError as e:
            check("error_unauthorized", "DENIED" in e.code or "UNAUTHORIZED" in e.code)

        # 14. Error: duplicate signup
        try:
            await client.signup(user_id="testuser1", password="anotherpass")
            check("error_conflict", False)
        except ShroudbAuthError as e:
            check("error_conflict", "STATE_ERROR" in e.code or "CONFLICT" in e.code)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
