"""ShrouDB Veil unified SDK integration test."""

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
    uri = os.environ.get("SHROUDB_VEIL_TEST_URI", "shroudb-veil://127.0.0.1:6999")
    db = ShrouDB(veil=uri)

    try:
        # Handshake sanity — every engine must answer HELLO.
        try:
            h = await db.veil.hello()
            check("hello: ok", True)
            check("hello: engine name", h.engine == "veil")
            check("hello: version not empty", isinstance(h.version, str) and len(h.version) > 0)
            check("hello: protocol", h.protocol == "RESP3/1")
        except Exception:
            check("hello: ok", False)

        # health
        try:
            result = await db.veil.health()
            check("health", result is not None)
        except Exception as e:
            check("health", False)
            print(f"    error: {e}")

        # index_create (use unique name per run to avoid "already exists")
        import time
        idx_name = f"test-idx-{int(time.time()) % 10000}"
        try:
            result = await db.veil.index_create(idx_name)
            check("index_create", result is not None)
        except ShrouDBError as e:
            # EXISTS is ok if index was created in a previous run
            check("index_create", "EXISTS" in str(e) or "exists" in str(e))
        except Exception as e:
            check("index_create", False)
            print(f"    error: {e}")

        # tokenize (veil expects base64-encoded plaintext)
        import base64
        plaintext_b64 = base64.b64encode(b"hello").decode()
        try:
            result = await db.veil.tokenize(idx_name, plaintext_b64)
            token = getattr(result, "token", None) or getattr(result, "tokens", None)
            check("tokenize", token is not None)
        except Exception as e:
            check("tokenize", False)
            print(f"    error: {e}")

        # put (store blind tokens for an entry)
        try:
            result = await db.veil.put(idx_name, "entry-1", plaintext_b64)
            check("put", result is not None)
        except Exception as e:
            check("put", False)
            print(f"    error: {e}")

        # search (search by token)
        try:
            result = await db.veil.search(idx_name, plaintext_b64)
            check("search", result is not None)
        except Exception as e:
            check("search", False)
            print(f"    error: {e}")

        # ── Blind (E2EE) operations ──────────────────────────────────
        import hmac
        import hashlib
        import json

        client_key = bytes([0x42] * 32)

        def blind_tokens(text: str) -> str:
            words = [w for w in __import__("re").split(r"[^a-z0-9]+", text.lower()) if w]
            word_tokens = sorted(set(f"w:{w}" for w in words))
            trigram_tokens = []
            for w in words:
                if len(w) >= 3:
                    for i in range(len(w) - 2):
                        trigram_tokens.append(f"t:{w[i:i+3]}")
            trigram_tokens = sorted(set(trigram_tokens))

            def do_hmac(token: str) -> str:
                return hmac.new(client_key, token.encode(), hashlib.sha256).hexdigest()

            token_set = {
                "words": [do_hmac(t) for t in word_tokens],
                "trigrams": [do_hmac(t) for t in trigram_tokens],
            }
            return base64.b64encode(json.dumps(token_set).encode()).decode()

        # put ... blind
        try:
            tokens_b64 = blind_tokens("hello world")
            result = await db.veil.put(idx_name, "blind-1", tokens_b64, blind=True)
            check("put_blind", result is not None)
        except Exception as e:
            check("put_blind", False)
            print(f"    error: {e}")

        # search ... blind (exact)
        try:
            query_b64 = blind_tokens("hello")
            result = await db.veil.search(idx_name, query_b64, mode="exact", blind=True)
            check("search_blind", result is not None and getattr(result, "matched", 0) >= 1)
        except Exception as e:
            check("search_blind", False)
            print(f"    error: {e}")

        # search ... blind with limit
        try:
            query_b64 = blind_tokens("hello")
            result = await db.veil.search(idx_name, query_b64, mode="contains", limit=5, blind=True)
            check("search_blind_with_limit", result is not None)
        except Exception as e:
            check("search_blind_with_limit", False)
            print(f"    error: {e}")

    finally:
        await db.close()
        check("close", True)

    print(f"\n{passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    asyncio.run(main())
