# frozen_string_literal: true

# ShrouDB core unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"

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

uri = ENV.fetch("SHROUDB_SHROUDB_TEST_URI", "shroudb://127.0.0.1:6399")
db = ShrouDB::Client.new(shroudb: uri)

begin
  # 1. Health
  result = db.shroudb.health
  check("health", !result.nil?)

  # 2. Namespace create (required before PUT/GET in v1)
  begin
    db.shroudb.namespace_create("test-ns")
    check("namespace_create", true)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("namespace_create", ok)
    puts "    error: #{e.message}" unless ok
  end

  # 3. PUT
  begin
    db.shroudb.put("test-ns", "test-key", "test-value")
    check("put", true)
  rescue StandardError => e
    check("put", false)
    puts "    error: #{e.message}"
  end

  # 4. GET
  begin
    result = db.shroudb.get("test-ns", "test-key")
    check("get", !result.nil?)
  rescue StandardError => e
    check("get", false)
    puts "    error: #{e.message}"
  end

  # 5. DELETE
  begin
    db.shroudb.delete("test-ns", "test-key")
    check("delete", true)
  rescue StandardError => e
    check("delete", false)
    puts "    error: #{e.message}"
  end

  # 6. Error: GET after delete
  begin
    db.shroudb.get("test-ns", "test-key")
    check("error_after_delete", false)
    puts "    expected ShrouDB::Error but succeeded"
  rescue ShrouDB::Error
    check("error_after_delete", true)
  rescue StandardError => e
    check("error_after_delete", false)
    puts "    unexpected error type: #{e.class}: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
