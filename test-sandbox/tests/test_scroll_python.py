"""ShrouDB Scroll unified SDK integration test."""

import asyncio
import base64
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
    uri = os.environ.get("SHROUDB_SCROLL_TEST_URI", "shroudb-scroll://127.0.0.1:7200")
    db = ShrouDB(scroll=uri)

    log = f"sandbox-log-py-{int(time.time()) % 100000}"
    group = "workers"

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.scroll.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "scroll")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            await db.scroll.health()
            check("health", True)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # ping
        try:
            pong = await db.scroll.ping()
            check("ping", pong is not None)
        except Exception as e:
            check("ping", False)
            print(f"    error: {e}")

        # append (creates the log on first call)
        payload_b64 = base64.b64encode(b"hello scroll").decode()
        try:
            result = await db.scroll.append(log, payload_b64)
            first_offset = getattr(result, "offset", None)
            check("append: first", first_offset == 0)
        except Exception as e:
            check("append: first", False)
            print(f"    error: {e}")

        # append (second entry)
        try:
            result = await db.scroll.append(log, base64.b64encode(b"second").decode())
            second_offset = getattr(result, "offset", None)
            check("append: second", second_offset == 1)
        except Exception as e:
            check("append: second", False)
            print(f"    error: {e}")

        # read (range)
        try:
            result = await db.scroll.read(log, 0, 10)
            entries = getattr(result, "entries", [])
            check("read: count", len(entries) == 2)
            check("read: payload roundtrip", base64.b64decode(entries[0].payload_b64) == b"hello scroll")
        except Exception as e:
            check("read: count", False)
            print(f"    error: {e}")

        # create_group
        try:
            await db.scroll.create_group(log, group, "earliest")
            check("create_group", True)
        except Exception as e:
            check("create_group", False)
            print(f"    error: {e}")

        # read_group
        try:
            result = await db.scroll.read_group(log, group, "reader-1", 10)
            entries = getattr(result, "entries", [])
            check("read_group: count", len(entries) == 2)
        except Exception as e:
            check("read_group: count", False)
            print(f"    error: {e}")

        # ack
        try:
            await db.scroll.ack(log, group, 0)
            await db.scroll.ack(log, group, 1)
            check("ack", True)
        except Exception as e:
            check("ack", False)
            print(f"    error: {e}")

        # log_info
        try:
            info = await db.scroll.log_info(log)
            check("log_info: entries_minted", info.entries_minted == 2)
            check("log_info: has group", group in (info.groups or []))
        except Exception as e:
            check("log_info", False)
            print(f"    error: {e}")

        # group_info
        try:
            info = await db.scroll.group_info(log, group)
            check("group_info: cursor", info.last_delivered_offset == 1)
            check("group_info: pending_count", info.pending_count == 0)
        except Exception as e:
            check("group_info", False)
            print(f"    error: {e}")

        # delete_group
        try:
            await db.scroll.delete_group(log, group)
            check("delete_group", True)
        except Exception as e:
            check("delete_group", False)
            print(f"    error: {e}")

        # delete_log
        try:
            await db.scroll.delete_log(log)
            check("delete_log", True)
        except Exception as e:
            check("delete_log", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
