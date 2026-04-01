"""ShrouDB Sigil unified SDK integration test."""

import asyncio
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
    uri = os.environ.get("SHROUDB_SIGIL_TEST_URI", "shroudb-sigil://127.0.0.1:6299")
    db = ShrouDB(sigil=uri)

    schema_name = f"test-schema-{int(time.time()) % 10000}"
    envelope_id = "test-envelope-1"
    user_id = "test-user-1"

    try:
        # health
        try:
            result = await db.sigil.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # schema_register (with credential field for verify/session tests)
        try:
            schema = {
                "fields": [
                    {"name": "username", "field_type": "string", "annotations": {"index": True}},
                    {"name": "password", "field_type": "string", "annotations": {"credential": True}},
                ],
            }
            result = await db.sigil.schema_register(schema_name, schema)
            check("schema_register", result is not None)
        except ShrouDBError as e:
            ok = "EXISTS" in str(e) or "exists" in str(e).lower()
            check("schema_register", ok)
            if not ok:
                print(f"    error: {e}")
        except Exception as e:
            check("schema_register", False)
            print(f"    error: {e}")

        # schema_list
        try:
            result = await db.sigil.schema_list()
            check("schema_list", result is not None)
        except Exception as e:
            check("schema_list", False)
            print(f"    error: {e}")

        # envelope_create
        try:
            result = await db.sigil.envelope_create(schema_name, envelope_id, {"username": "testuser", "password": "s3cret123!"})
            check("envelope_create", result is not None)
        except ShrouDBError as e:
            ok = "EXISTS" in str(e) or "exists" in str(e).lower()
            check("envelope_create", ok)
            if not ok:
                print(f"    error: {e}")
        except Exception as e:
            check("envelope_create", False)
            print(f"    error: {e}")

        # envelope_get
        try:
            result = await db.sigil.envelope_get(schema_name, envelope_id)
            check("envelope_get", result is not None)
        except Exception as e:
            check("envelope_get", False)
            print(f"    error: {e}")

        # envelope_verify
        try:
            result = await db.sigil.envelope_verify(schema_name, envelope_id, "password", "s3cret123!")
            check("envelope_verify", result is not None and result.valid is True)
        except Exception as e:
            check("envelope_verify", False)
            print(f"    error: {e}")

        # envelope_delete
        try:
            result = await db.sigil.envelope_delete(schema_name, envelope_id)
            check("envelope_delete", result is not None)
        except Exception as e:
            check("envelope_delete", False)
            print(f"    error: {e}")

        # user_create (sugar for envelope_create)
        try:
            result = await db.sigil.user_create(schema_name, user_id, {"username": "testuser2", "password": "s3cret123!"})
            check("user_create", result is not None)
        except ShrouDBError as e:
            ok = "EXISTS" in str(e) or "exists" in str(e).lower()
            check("user_create", ok)
            if not ok:
                print(f"    error: {e}")
        except Exception as e:
            check("user_create", False)
            print(f"    error: {e}")

        # user_verify (sugar for envelope_verify with implicit field)
        try:
            result = await db.sigil.user_verify(schema_name, user_id, "s3cret123!")
            check("user_verify", result is not None and result.valid is True)
        except Exception as e:
            check("user_verify", False)
            print(f"    error: {e}")

        # session_create
        try:
            result = await db.sigil.session_create(schema_name, user_id, "s3cret123!")
            check("session_create", result is not None and result.access_token != "")
        except Exception as e:
            check("session_create", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
