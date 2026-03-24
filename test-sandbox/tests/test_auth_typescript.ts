/**
 * ShrouDB Auth TypeScript HTTP client integration test.
 *
 * Exercises the generated client against a live ShrouDB Auth server.
 * Expects SHROUDB_AUTH_TEST_URL env var (e.g. http://127.0.0.1:4001).
 */

import { ShroudbAuthClient } from "./shroudb-auth-client/src/index.js";
import { ShroudbAuthError } from "./shroudb-auth-client/src/errors.js";

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

async function main(): Promise<void> {
  const baseUrl =
    process.env.SHROUDB_AUTH_TEST_URL ?? "http://127.0.0.1:4001";
  const client = new ShroudbAuthClient(baseUrl, "default");

  try {
    // 1. Health
    const h = await client.health();
    check("health", h.status === "healthy" || h.status === "ok" || h.status === "OK");

    // 2. Signup
    const signup = await client.signup("testpass123", "testuser_ts");
    check(
      "signup",
      signup.access_token != null && signup.refresh_token != null,
    );
    const access = signup.access_token;
    const refresh = signup.refresh_token;

    // 3. Session (verify access token)
    client.accessToken = access;
    const session = await client.session();
    check("session", session.user_id === "testuser_ts");

    // 4. Login
    const login = await client.login("testpass123", "testuser_ts");
    check("login", login.access_token != null);

    // 5. Refresh
    client.refreshToken = refresh;
    const ref = await client.refresh();
    check("refresh", ref.access_token != null);

    // 6. Change password
    client.accessToken = login.access_token;
    await client.changePassword("newpass456", "testpass123");
    check("change_password", true);

    // 7. Login with new password
    const login2 = await client.login("newpass456", "testuser_ts");
    check("login_new_pass", login2.access_token != null);

    // 8. Forgot password
    const fp = await client.forgotPassword("testuser_ts");
    check("forgot_password", fp.reset_token != null);

    // 9. Reset password
    await client.resetPassword("resetpass789", fp.reset_token!);
    check("reset_password", true);

    // 10. Login after reset
    const login3 = await client.login("resetpass789", "testuser_ts");
    check("login_after_reset", login3.access_token != null);

    // 11. Logout
    client.accessToken = login3.access_token;
    client.refreshToken = login3.refresh_token;
    await client.logout();
    check("logout", true);

    // 12. JWKS
    const jwks = await client.jwks();
    check("jwks", jwks.keys != null);

    // 13. Error: wrong password
    try {
      await client.login("wrongpass", "testuser_ts");
      check("error_unauthorized", false);
    } catch (e) {
      check("error_unauthorized", e instanceof ShroudbAuthError);
    }

    // 14. Error: duplicate signup
    try {
      await client.signup("anotherpass", "testuser_ts");
      check("error_conflict", false);
    } catch (e) {
      check("error_conflict", e instanceof ShroudbAuthError);
    }
  } finally {
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
