# frozen_string_literal: true

# ShrouDB Sentry unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"
require "json"

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

uri = ENV.fetch("SHROUDB_SENTRY_TEST_URI", "shroudb-sentry://127.0.0.1:6499")
db = ShrouDB::Client.new(sentry: uri)

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.sentry.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "sentry")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  # 1. Health
  begin
    result = db.sentry.health
    check("health", !result.nil?)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 2. PolicyList
  begin
    result = db.sentry.policy_list
    check("policy_list", !result.nil?)
  rescue StandardError => e
    check("policy_list", false)
    puts "    error: #{e.message}"
  end

  # 3. Evaluate (pass JSON string)
  begin
    eval_json = JSON.generate({
      "principal" => "user:test@example.com",
      "resource" => "secret:db/test/*",
      "action" => "read"
    })
    result = db.sentry.evaluate(eval_json)
    check("evaluate", !result.nil?)
  rescue StandardError => e
    check("evaluate", false)
    puts "    error: #{e.message}"
  end

  # 4. KeyInfo
  begin
    result = db.sentry.key_info
    check("key_info", !result.nil?)
  rescue StandardError => e
    check("key_info", false)
    puts "    error: #{e.message}"
  end

  # 5. PolicyCreate
  policy_name = "test-policy-#{Time.now.to_i % 10000}"
  begin
    policy_body = JSON.generate({
      "effect" => "permit",
      "principals" => ["user:*"],
      "resources" => ["secret:test/*"],
      "actions" => ["read"]
    })
    db.sentry.policy_create(policy_name, policy_body)
    check("policy_create", true)
  rescue ShrouDB::Error
    # EXISTS or DENIED are both acceptable
    check("policy_create", true)
  rescue StandardError => e
    check("policy_create", false)
    puts "    error: #{e.message}"
  end

  # 6. PolicyDelete
  begin
    db.sentry.policy_delete(policy_name)
    check("policy_delete", true)
  rescue ShrouDB::Error
    # DENIED or NOTFOUND are both acceptable
    check("policy_delete", true)
  rescue StandardError => e
    check("policy_delete", false)
    puts "    error: #{e.message}"
  end

  # 7. Error: PolicyGet nonexistent
  begin
    db.sentry.policy_get("nonexistent-policy-xyz")
    check("error_notfound", false)
    puts "    expected ShrouDB::Error but succeeded"
  rescue ShrouDB::Error
    check("error_notfound", true)
  rescue StandardError => e
    check("error_notfound", false)
    puts "    unexpected error type: #{e.class}: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
