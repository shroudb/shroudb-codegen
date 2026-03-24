# frozen_string_literal: true

# ShrouDB Auth HTTP Ruby client integration test.

require "shroudb_auth"

$passed = 0
$failed = 0

def check(name, condition)
  if condition
    $passed += 1
    puts "  PASS  #{name}"
  else
    $failed += 1
    puts "  FAIL  #{name}"
  end
end

base_url = ENV.fetch("SHROUDB_AUTH_TEST_URL", "http://127.0.0.1:4001")
client = ShroudbAuth::Client.new(base_url: base_url, keyspace: "default")

begin
  # 1. Health
  h = client.health
  check("health", %w[healthy ok OK].include?(h.status))

  # 2. Signup
  signup = client.signup(user_id: "testuser_rb", password: "testpass123")
  check("signup", !signup.access_token.nil? && !signup.refresh_token.nil?)
  access = signup.access_token
  refresh = signup.refresh_token

  # 3. Session (verify access token)
  client.access_token = access
  session = client.session
  check("session", session.user_id == "testuser_rb")

  # 4. Login
  login = client.login(user_id: "testuser_rb", password: "testpass123")
  check("login", !login.access_token.nil?)

  # 5. Refresh
  client.refresh_token = refresh
  ref = client.refresh
  check("refresh", !ref.access_token.nil?)

  # 6. Change password
  client.access_token = login.access_token
  client.change_password(old_password: "testpass123", new_password: "newpass456")
  check("change_password", true)

  # 7. Login with new password
  login2 = client.login(user_id: "testuser_rb", password: "newpass456")
  check("login_new_pass", !login2.access_token.nil?)

  # 8. Forgot password
  fp = client.forgot_password(user_id: "testuser_rb")
  check("forgot_password", !fp.reset_token.nil?)

  # 9. Reset password
  client.reset_password(token: fp.reset_token, new_password: "resetpass789")
  check("reset_password", true)

  # 10. Login after reset
  login3 = client.login(user_id: "testuser_rb", password: "resetpass789")
  check("login_after_reset", !login3.access_token.nil?)

  # 11. Logout
  client.access_token = login3.access_token
  client.refresh_token = login3.refresh_token
  client.logout
  check("logout", true)

  # 12. JWKS
  jwks = client.jwks
  check("jwks", !jwks.keys.nil?)

  # 13. Error: wrong password
  begin
    client.login(user_id: "testuser_rb", password: "wrongpass")
    check("error_unauthorized", false)
  rescue ShroudbAuth::Error => e
    check("error_unauthorized", e.code.include?("DENIED") || e.code.include?("UNAUTHORIZED"))
  end

  # 14. Error: duplicate signup
  begin
    client.signup(user_id: "testuser_rb", password: "anotherpass")
    check("error_conflict", false)
  rescue ShroudbAuth::Error => e
    check("error_conflict", e.code.include?("STATE_ERROR") || e.code.include?("CONFLICT"))
  end

ensure
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
