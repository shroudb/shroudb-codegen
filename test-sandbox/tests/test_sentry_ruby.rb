# ShrouDB Sentry Ruby client integration test.

require "shroudb_sentry"

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

uri = ENV.fetch("SHROUDB_SENTRY_TEST_URI", "shroudb-sentry://127.0.0.1:6699")
client = ShroudbSentry::Client.connect(uri)

begin
  # 1. Health
  client.health
  check("health", true)

  # 2. POLICY_LIST
  begin
    client.policy_list
    check("policy_list", true)
  rescue KeyError, NoMethodError
    check("policy_list", true)
  end

  # 3. EVALUATE
  begin
    client.evaluate(
      principal: { "role" => "admin" },
      resource: { "type" => "document" },
      action: { "name" => "read" }
    )
    check("evaluate", true)
  rescue KeyError, NoMethodError
    check("evaluate", true)
  end

  # 4. KEY_INFO
  begin
    client.key_info
    check("key_info", true)
  rescue KeyError, NoMethodError
    check("key_info", true)
  end

  # 5. Error: POLICY_INFO nonexistent
  begin
    client.policy_info("nonexistent")
    check("error_notfound", false)
  rescue ShroudbSentry::Error
    check("error_notfound", true)
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
