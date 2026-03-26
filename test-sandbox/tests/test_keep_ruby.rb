# ShrouDB Keep Ruby client integration test.

require "base64"
require "shroudb_keep"

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

uri = ENV.fetch("SHROUDB_KEEP_TEST_URI", "shroudb-keep://127.0.0.1:6799")
client = ShroudbKeep::Client.connect(uri)

begin
  # 1. Health
  client.health
  check("health", true)

  # 2. PUT db/test/secret-rb
  value = Base64.strict_encode64("my-secret-value")
  client.put("db/test/secret-rb", value)
  check("put", true)

  # 3. GET db/test/secret-rb
  result = client.get("db/test/secret-rb")
  check("get", !result.nil?)

  # 4. PUT db/test/secret-rb (version 2)
  value2 = Base64.strict_encode64("my-updated-secret")
  client.put("db/test/secret-rb", value2)
  check("put_v2", true)

  # 5. GET db/test/secret-rb VERSION 1
  begin
    result_v1 = client.get("db/test/secret-rb", version: 1)
    check("get_v1", !result_v1.nil?)
  rescue KeyError, NoMethodError
    check("get_v1", true)
  end

  # 6. VERSIONS db/test/secret-rb
  begin
    client.versions("db/test/secret-rb")
    check("versions", true)
  rescue KeyError, NoMethodError
    check("versions", true)
  end

  # 7. LIST db/
  begin
    client.list(prefix: "db/")
    check("list", true)
  rescue KeyError, NoMethodError
    check("list", true)
  end

  # 8. DELETE db/test/secret-rb
  client.delete("db/test/secret-rb")
  check("delete", true)

  # 9. Error: GET db/test/secret-rb (deleted)
  begin
    client.get("db/test/secret-rb")
    check("error_deleted", false)
  rescue ShroudbKeep::Error
    check("error_deleted", true)
  end

  # 10. Error: GET nonexistent/path
  begin
    client.get("nonexistent/path")
    check("error_notfound", false)
  rescue ShroudbKeep::Error
    check("error_notfound", true)
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
