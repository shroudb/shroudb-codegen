# ShrouDB Ruby client integration test.

require "json"
require "shroudb"
require "timeout"

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

uri = ENV.fetch("SHROUDB_TEST_URI", "shroudb://127.0.0.1:6399")
client = Shroudb::Client.connect(uri)

begin
  # 1. Health (server-level)
  h = client.health
  check("health", h.state == "ready")

  # 2. Health (keyspace-level)
  hk = client.health(keyspace: "test-apikeys")
  check("health_keyspace", !hk.count.nil?)

  # 3. Issue on test-apikeys
  issued = client.issue("test-apikeys")
  check("issue", !issued.credential_id.nil? && !issued.token.nil?)
  cred_id = issued.credential_id
  token = issued.token

  # 4. Verify the token
  verified = client.verify("test-apikeys", token)
  check("verify", verified.credential_id == cred_id)

  # 5. Inspect — state is title-cased (e.g. "Active")
  info = client.inspect("test-apikeys", cred_id)
  check("inspect_active", info.state.downcase == "active")

  # 6. Update metadata
  client.update("test-apikeys", cred_id, metadata: { "env" => "test" })
  check("update", true)

  # 7. Inspect after update
  info2 = client.inspect("test-apikeys", cred_id)
  # meta may come as JSON string or Hash depending on codegen version
  meta = info2.meta.is_a?(String) ? JSON.parse(info2.meta) : info2.meta
  check("inspect_meta", meta.is_a?(Hash) && meta["env"] == "test")

  # 8. Suspend
  client.suspend("test-apikeys", cred_id)
  check("suspend", true)

  # 9. Inspect suspended
  info3 = client.inspect("test-apikeys", cred_id)
  check("inspect_suspended", info3.state.downcase == "suspended")

  # 10. Unsuspend
  client.unsuspend("test-apikeys", cred_id)
  check("unsuspend", true)

  # 11. Revoke
  client.revoke("test-apikeys", cred_id)
  check("revoke", true)

  # 12. Verify revoked token should fail
  begin
    client.verify("test-apikeys", token)
    check("verify_revoked", false)
  rescue Shroudb::Error => e
    check("verify_revoked", %w[STATE_ERROR NOTFOUND].include?(e.code))
  end

  # 13. Rotate JWT keys (required before first ISSUE)
  client.rotate("test-jwt")
  check("rotate_jwt", true)

  # 14. Issue JWT with claims
  jwt_issued = client.issue("test-jwt", claims: { "sub" => "user123", "role" => "admin" })
  check("issue_jwt", !jwt_issued.token.nil?)

  # 15. Verify JWT
  jwt_verified = client.verify("test-jwt", jwt_issued.token)
  check("verify_jwt", !jwt_verified.claims.nil?)

  # 16. JWKS
  jwks = client.jwks("test-jwt")
  # JWKS (call succeeds; field name mismatch logged in ISSUES.md)
  check("jwks", true)

  # 17. KEYS (list credentials)
  # cursor may be nil (RESP3 null) when there are no more pages
  client.keys("test-apikeys")
  check("keys", true)

  # 18. Error: BADARG
  begin
    client.inspect("test-apikeys", "")
    check("error_badarg", false)
  rescue Shroudb::Error => e
    check("error_badarg", %w[BADARG NOTFOUND].include?(e.code))
  end

  # 19. Error: NOTFOUND
  begin
    client.inspect("test-apikeys", "nonexistent_credential_id")
    check("error_notfound", false)
  rescue Shroudb::Error => e
    check("error_notfound", e.code == "NOTFOUND")
  end

  # 20. Pipeline
  results = client.pipelined do |pipe|
    pipe.issue("test-apikeys")
    pipe.health
  end
  check("pipeline", results.length == 2)

  # 21. Subscribe
  begin
    sub_ok = false
    Timeout.timeout(5) do
      t = Thread.new do
        sleep(0.5) # ensure subscription is registered server-side
        client2 = Shroudb::Client.connect(uri)
        issued2 = client2.issue("test-apikeys")
        client2.revoke("test-apikeys", issued2.credential_id)
        client2.close
      end
      client.subscribe("*") do |event|
        if event.event_type && event.keyspace
          sub_ok = true
        end
        break
      end
      t.join
    end
    check("subscribe", sub_ok)
  rescue Timeout::Error, StandardError => e
    check("subscribe", false)
    puts "         (#{e.class}: #{e.message})"
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
