# ShrouDB Ruby client integration test.
#
# Exercises the generated client against a live ShrouDB server.
# Expects SHROUDB_TEST_URI env var (e.g. shroudb://127.0.0.1:6399).

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
  check("health", h.status == "OK")

  # 2. Health (keyspace-level)
  hk = client.health(keyspace: "test-apikeys")
  check("health_keyspace", hk.status == "OK")

  # 3. Issue on test-apikeys
  issued = client.issue("test-apikeys")
  check("issue", !issued.credential_id.nil? && !issued.token.nil?)
  cred_id = issued.credential_id
  token = issued.token

  # 4. Verify the token
  verified = client.verify("test-apikeys", token)
  check("verify", verified.credential_id == cred_id)

  # 5. Inspect
  info = client.inspect("test-apikeys", cred_id)
  check("inspect_active", info.state == "active")

  # 6. Update metadata
  client.update("test-apikeys", cred_id, metadata: { "env" => "test" })
  check("update", true)

  # 7. Inspect after update
  info2 = client.inspect("test-apikeys", cred_id)
  check("inspect_meta", info2.meta.is_a?(Hash) && info2.meta["env"] == "test")

  # 8. Suspend
  client.suspend("test-apikeys", cred_id)
  check("suspend", true)

  # 9. Inspect suspended
  info3 = client.inspect("test-apikeys", cred_id)
  check("inspect_suspended", info3.state == "suspended")

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

  # 13. Issue JWT with claims
  jwt_issued = client.issue("test-jwt", claims: { "sub" => "user123", "role" => "admin" })
  check("issue_jwt", !jwt_issued.token.nil?)

  # 14. Verify JWT
  jwt_verified = client.verify("test-jwt", jwt_issued.token)
  check("verify_jwt", !jwt_verified.claims.nil?)

  # 15. JWKS
  jwks = client.jwks("test-jwt")
  check("jwks", jwks.keys.is_a?(Array) && !jwks.keys.empty?)

  # 16. KEYS (list credentials)
  keys_result = client.keys("test-apikeys")
  check("keys", !keys_result.cursor.nil?)

  # 17. Error: BADARG
  begin
    client.inspect("test-apikeys", "")
    check("error_badarg", false)
  rescue Shroudb::Error => e
    check("error_badarg", e.code == "BADARG")
  end

  # 18. Error: NOTFOUND
  begin
    client.inspect("test-apikeys", "nonexistent_credential_id")
    check("error_notfound", false)
  rescue Shroudb::Error => e
    check("error_notfound", e.code == "NOTFOUND")
  end

  # 19. Pipeline
  pipe = client.pipeline
  pipe.issue("test-apikeys")
  pipe.health
  results = pipe.execute
  check("pipeline", results.length == 2)

  # 20. Subscribe
  begin
    sub_ok = false
    Timeout.timeout(5) do
      client.subscribe("*") do |event|
        # Trigger an event from a second connection in a thread
        Thread.new do
          client2 = Shroudb::Client.connect(uri)
          client2.issue("test-apikeys")
          client2.close
        end.join

        if event.event_type && event.keyspace
          sub_ok = true
        end
        break  # one event is enough
      end
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
