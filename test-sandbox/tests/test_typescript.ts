/**
 * ShrouDB TypeScript client integration test.
 *
 * Exercises the generated client against a live ShrouDB server.
 * Expects SHROUDB_TEST_URI env var (e.g. shroudb://127.0.0.1:6399).
 */

import { ShroudbClient } from "./src/index";
import { ShroudbError } from "./src/errors";
import { Subscription } from "./src/subscription";

let passed = 0;
let failed = 0;

function check(name: string, condition: boolean): void {
  if (condition) {
    passed++;
    console.log(`  PASS  ${name}`);
  } else {
    failed++;
    console.log(`  FAIL  ${name}`);
  }
}

const uri = process.env.SHROUDB_TEST_URI || "shroudb://127.0.0.1:6399";
const client = await ShroudbClient.connect(uri);

try {
  // 1. Health (server-level)
  const h = await client.health();
  check("health", h.status === "OK");

  // 2. Health (keyspace-level)
  const hk = await client.health("test-apikeys");
  check("health_keyspace", hk.status === "OK");

  // 3. Issue on test-apikeys
  const issued = await client.issue("test-apikeys");
  check("issue", !!issued.credentialId && !!issued.token);
  const credId = issued.credentialId;
  const token = issued.token;

  // 4. Verify the token
  const verified = await client.verify("test-apikeys", token);
  check("verify", verified.credentialId === credId);

  // 5. Inspect
  const info = await client.inspect("test-apikeys", credId);
  check("inspect_active", info.state === "active");

  // 6. Update metadata
  await client.update("test-apikeys", credId, { metadata: { env: "test" } });
  check("update", true);

  // 7. Inspect after update
  const info2 = await client.inspect("test-apikeys", credId);
  check("inspect_meta", info2.meta?.env === "test");

  // 8. Suspend
  await client.suspend("test-apikeys", credId);
  check("suspend", true);

  // 9. Inspect suspended
  const info3 = await client.inspect("test-apikeys", credId);
  check("inspect_suspended", info3.state === "suspended");

  // 10. Unsuspend
  await client.unsuspend("test-apikeys", credId);
  check("unsuspend", true);

  // 11. Revoke
  await client.revoke("test-apikeys", credId);
  check("revoke", true);

  // 12. Verify revoked token should fail
  try {
    await client.verify("test-apikeys", token);
    check("verify_revoked", false);
  } catch (e) {
    check(
      "verify_revoked",
      e instanceof ShroudbError &&
        (e.code === "STATE_ERROR" || e.code === "NOTFOUND")
    );
  }

  // 13. Issue JWT with claims
  const jwtIssued = await client.issue("test-jwt", {
    claims: { sub: "user123", role: "admin" },
  });
  check("issue_jwt", !!jwtIssued.token);

  // 14. Verify JWT
  const jwtVerified = await client.verify("test-jwt", jwtIssued.token);
  check("verify_jwt", jwtVerified.claims != null);

  // 15. JWKS
  const jwks = await client.jwks("test-jwt");
  check("jwks", Array.isArray(jwks.keys) && jwks.keys.length > 0);

  // 16. KEYS (list credentials)
  const keysResult = await client.keys("test-apikeys");
  check("keys", keysResult.cursor != null);

  // 17. Error: BADARG
  try {
    await client.inspect("test-apikeys", "");
    check("error_badarg", false);
  } catch (e) {
    check("error_badarg", e instanceof ShroudbError && e.code === "BADARG");
  }

  // 18. Error: NOTFOUND
  try {
    await client.inspect("test-apikeys", "nonexistent_credential_id");
    check("error_notfound", false);
  } catch (e) {
    check(
      "error_notfound",
      e instanceof ShroudbError && e.code === "NOTFOUND"
    );
  }

  // 19. Pipeline
  const pipe = client.pipeline();
  pipe.issue("test-apikeys");
  pipe.health();
  const results = await pipe.execute();
  check("pipeline", results.length === 2);

  // 20. Subscribe
  try {
    let subOk = false;
    const sub: Subscription = await client.subscribe("*");

    // Trigger an event from a second client
    const client2 = await ShroudbClient.connect(uri);
    await client2.issue("test-apikeys");
    await client2.close();

    const timeout = new Promise<void>((_, reject) =>
      setTimeout(() => reject(new Error("timeout")), 5000)
    );
    const readEvent = (async () => {
      for await (const event of sub) {
        if (event.eventType && event.keyspace) {
          subOk = true;
        }
        break;
      }
    })();

    await Promise.race([readEvent, timeout]);
    sub.close();
    check("subscribe", subOk);
  } catch (e: any) {
    check("subscribe", false);
    console.log(`         (${e.message})`);
  }
} finally {
  await client.close();
  check("close", true);
}

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
