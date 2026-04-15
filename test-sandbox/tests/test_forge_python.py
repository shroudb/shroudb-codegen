"""ShrouDB Forge unified SDK integration test."""

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
    uri = os.environ.get("SHROUDB_FORGE_TEST_URI", "shroudb-forge://127.0.0.1:6699")
    db = ShrouDB(forge=uri)

    try:
        # ca_list serves as health check (forge has no RESP3 HEALTH command)
        try:
            result = await db.forge.ca_list()
            check("health_via_ca_list", result is not None)
        except Exception as e:
            check("health_via_ca_list", False)
            print(f"    error: {e}")

        # ca_create — exercises the `SUBJECT` keyword-prefix wire path.
        try:
            result = await db.forge.ca_create(
                "codegen-new-ca", "ecdsa-p256", "CN=Codegen New CA", ttl_days=30
            )
            check("ca_create", result is not None and result.name == "codegen-new-ca")
        except Exception as e:
            check("ca_create", False)
            print(f"    error: {e}")

        # ca_info
        try:
            result = await db.forge.ca_info("test-ca")
            check("ca_info", result is not None)
        except Exception as e:
            check("ca_info", False)
            print(f"    error: {e}")

        # ca_list
        try:
            result = await db.forge.ca_list()
            check("ca_list", result is not None)
        except Exception as e:
            check("ca_list", False)
            print(f"    error: {e}")

        # issue
        serial = None
        try:
            result = await db.forge.issue("test-ca", "CN=test.example.com", "server")
            cert = getattr(result, "certificate_pem", None) or getattr(result, "certificate", None)
            serial = getattr(result, "serial", None)
            check("issue", cert is not None and serial is not None)
        except Exception as e:
            check("issue", False)
            print(f"    error: {e}")

        # inspect (use serial from issue)
        if serial:
            try:
                result = await db.forge.inspect("test-ca", serial)
                check("inspect", result is not None and result.serial == serial)
            except Exception as e:
                check("inspect", False)
                print(f"    error: {e}")
        else:
            check("inspect", False)
            print("    skipped: no serial from issue")

        # list_certs
        try:
            result = await db.forge.list_certs("test-ca")
            check("list_certs", result is not None)
        except Exception as e:
            check("list_certs", False)
            print(f"    error: {e}")

        # revoke (use serial from issue)
        if serial:
            try:
                result = await db.forge.revoke("test-ca", serial)
                check("revoke", result is not None)
            except Exception as e:
                check("revoke", False)
                print(f"    error: {e}")
        else:
            check("revoke", False)
            print("    skipped: no serial from issue")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
