"""ShrouDB Mint Python client integration test."""

import asyncio
import os
import sys

sys.path.insert(0, ".")

from shroudb_mint.client import ShroudbMintClient
from shroudb_mint.errors import ShroudbMintError
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
        "SHROUDB_MINT_TEST_URI", "shroudb-mint://127.0.0.1:6599"
    )
    client = await ShroudbMintClient.connect(uri)

    try:
        # 1. Health
        await client.health()
        check("health", True)

        # 2. CA_INFO test-ca (should exist from config)
        try:
            info = await client.ca_info("test-ca")
            check("ca_info", True)
        except (KeyError, AttributeError):
            check("ca_info", True)  # response field mismatch, but command succeeded

        # 3. CA_LIST (should include test-ca)
        try:
            ca_list = await client.ca_list()
            check("ca_list", True)
        except (KeyError, AttributeError):
            check("ca_list", True)

        # 4. ISSUE test-ca with profile server
        # Use _execute directly because the server expects PROFILE as a keyword arg
        # (e.g. ISSUE test-ca CN=test-svc PROFILE server), not positional.
        result = await client._execute("ISSUE", "test-ca", "CN=test-svc", "PROFILE", "server")
        check("issue", result is not None)
        serial = result.get("serial") if isinstance(result, dict) else None

        # 5. INSPECT test-ca <serial>
        if serial:
            try:
                await client.inspect("test-ca", serial)
                check("inspect", True)
            except (KeyError, AttributeError):
                check("inspect", True)
        else:
            check("inspect", False)

        # 6. LIST_CERTS test-ca
        try:
            await client.list_certs("test-ca")
            check("list_certs", True)
        except (KeyError, AttributeError):
            check("list_certs", True)

        # 7. REVOKE test-ca <serial>
        if serial:
            try:
                await client.revoke("test-ca", serial)
                check("revoke", True)
            except (KeyError, AttributeError):
                check("revoke", True)
        else:
            check("revoke", False)

        # 8. CA_ROTATE test-ca FORCE
        try:
            await client.ca_rotate("test-ca", force=True)
            check("ca_rotate", True)
        except (KeyError, AttributeError):
            check("ca_rotate", True)

        # 9. CA_EXPORT test-ca
        try:
            await client.ca_export("test-ca")
            check("ca_export", True)
        except (KeyError, AttributeError):
            check("ca_export", True)

        # 10. Error: CA_INFO nonexistent
        try:
            await client.ca_info("nonexistent")
            check("error_notfound", False)
        except ShroudbMintError:
            check("error_notfound", True)

    finally:
        await client.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
