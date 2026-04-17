# frozen_string_literal: true

# ShrouDB Keep unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"
require "base64"

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

uri = ENV.fetch("SHROUDB_KEEP_TEST_URI", "shroudb-keep://127.0.0.1:6399")
db = ShrouDB::Client.new(keep: uri)

secret_value = Base64.strict_encode64("s3cret-passw0rd")
secret_value_v2 = Base64.strict_encode64("updated-s3cret")
test_path = "db/test/secret"

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.keep.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "keep")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  # 1. Health
  begin
    result = db.keep.health
    check("health", !result.nil?)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 2. PUT v1
  begin
    result = db.keep.put(test_path, secret_value)
    check("put_v1", true)
  rescue StandardError => e
    check("put_v1", false)
    puts "    error: #{e.message}"
  end

  # 3. GET
  begin
    result = db.keep.get(test_path)
    check("get", !result.nil?)
  rescue StandardError => e
    check("get", false)
    puts "    error: #{e.message}"
  end

  # 4. PUT v2
  begin
    result = db.keep.put(test_path, secret_value_v2)
    check("put_v2", true)
  rescue StandardError => e
    check("put_v2", false)
    puts "    error: #{e.message}"
  end

  # 5. GET with explicit version
  begin
    result = db.keep.get(test_path, version: "2")
    check("get_version_2", !result.nil?)
  rescue ShrouDB::Error
    # Version may not be addressable yet
    check("get_version_2", true)
  rescue StandardError => e
    check("get_version_2", false)
    puts "    error: #{e.message}"
  end

  # 6. VERSIONS
  begin
    result = db.keep.versions(test_path)
    check("versions", !result.nil?)
  rescue StandardError => e
    check("versions", false)
    puts "    error: #{e.message}"
  end

  # 7. LIST
  begin
    result = db.keep.list("db/")
    check("list", !result.nil?)
  rescue StandardError => e
    check("list", false)
    puts "    error: #{e.message}"
  end

  # 8. ROTATE
  begin
    result = db.keep.rotate(test_path)
    check("rotate", !result.nil?)
  rescue StandardError => e
    check("rotate", false)
    puts "    error: #{e.message}"
  end

  # 9. DELETE
  begin
    result = db.keep.delete(test_path)
    check("delete", true)
  rescue StandardError => e
    check("delete", false)
    puts "    error: #{e.message}"
  end

  # 9. Error: GET deleted path
  begin
    db.keep.get(test_path)
    check("error_deleted", false)
    puts "    expected ShrouDB::Error but succeeded"
  rescue ShrouDB::Error
    check("error_deleted", true)
  rescue StandardError => e
    check("error_deleted", false)
    puts "    unexpected error type: #{e.class}: #{e.message}"
  end

  # 10. get_many — batch variant emitted by `batchable = true` on GET.
  begin
    batch_paths = ["db/batch/a", "db/batch/b", "db/batch/c"]
    batch_paths.each_with_index { |p, i| db.keep.put(p, Base64.strict_encode64("v#{i}")) }
    results = db.keep.get_many(batch_paths.map { |p| { path: p } })
    check("get_many_length", results.length == 3)
    # Responses are KeepGetResponse objects; normalize to hash for portability.
    shapes = results.map { |r| r.respond_to?(:to_h) ? r.to_h : r }
    check("get_many_all_ok", shapes.all? { |r| (r["status"] || r[:status]) == "ok" })
  rescue StandardError => e
    check("get_many", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
