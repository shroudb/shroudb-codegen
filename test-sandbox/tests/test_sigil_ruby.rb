# frozen_string_literal: true

# ShrouDB Sigil unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"
require "json"

$passed = 0
$failed = 0

def check(name, ok)
  if ok
    $passed += 1
    puts "  PASS  #{name}"
  else
    $failed += 1
    puts "  FAIL  #{name}"
  end
end

uri = ENV.fetch("SHROUDB_SIGIL_TEST_URI", "shroudb-sigil://127.0.0.1:6299")
db = ShrouDB::Client.new(sigil: uri)

schema_name = "test-schema-#{Time.now.to_i % 10000}"
envelope_id = "test-envelope-1"
user_id = "test-user-1"

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.sigil.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "sigil")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  # 1. Health
  begin
    result = db.sigil.health
    check("health", !result.nil?)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 1b. Ping — added in Sigil v2.1 to restore uniform meta-command coverage.
  begin
    db.sigil.ping
    check("ping", true)
  rescue StandardError => e
    check("ping", false)
    puts "    error: #{e.message}"
  end

  # 2. Schema register (with credential field for verify/session tests)
  begin
    schema = {
      "fields" => [
        { "name" => "username", "field_type" => "string", "kind" => { "type" => "index" } },
        { "name" => "password", "field_type" => "string", "kind" => { "type" => "credential" } }
      ]
    }
    result = db.sigil.schema_register(schema_name, schema)
    check("schema_register", !result.nil?)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("schema_register", ok)
    puts "    error: #{e.message}" unless ok
  rescue StandardError => e
    check("schema_register", false)
    puts "    error: #{e.message}"
  end

  # 3. Schema list
  begin
    result = db.sigil.schema_list
    check("schema_list", !result.nil?)
  rescue StandardError => e
    check("schema_list", false)
    puts "    error: #{e.message}"
  end

  # 4. Envelope create
  begin
    result = db.sigil.envelope_create(schema_name, envelope_id, { "username" => "testuser", "password" => "s3cret123!" })
    check("envelope_create", !result.nil?)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("envelope_create", ok)
    puts "    error: #{e.message}" unless ok
  rescue StandardError => e
    check("envelope_create", false)
    puts "    error: #{e.message}"
  end

  # 5. Envelope get
  begin
    result = db.sigil.envelope_get(schema_name, envelope_id)
    check("envelope_get", !result.nil?)
  rescue StandardError => e
    check("envelope_get", false)
    puts "    error: #{e.message}"
  end

  # 6. Envelope verify
  begin
    result = db.sigil.envelope_verify(schema_name, envelope_id, "password", "s3cret123!")
    check("envelope_verify", !result.nil? && result.valid == true)
  rescue StandardError => e
    check("envelope_verify", false)
    puts "    error: #{e.message}"
  end

  # 7. Envelope delete
  begin
    result = db.sigil.envelope_delete(schema_name, envelope_id)
    check("envelope_delete", !result.nil?)
  rescue StandardError => e
    check("envelope_delete", false)
    puts "    error: #{e.message}"
  end

  # 8. User create (sugar for envelope_create)
  begin
    result = db.sigil.user_create(schema_name, user_id, { "username" => "testuser2", "password" => "s3cret123!" })
    check("user_create", !result.nil?)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("user_create", ok)
    puts "    error: #{e.message}" unless ok
  rescue StandardError => e
    check("user_create", false)
    puts "    error: #{e.message}"
  end

  # 9. User verify
  begin
    result = db.sigil.user_verify(schema_name, user_id, "s3cret123!")
    check("user_verify", !result.nil? && result.valid == true)
  rescue StandardError => e
    check("user_verify", false)
    puts "    error: #{e.message}"
  end

  # 10. Session create
  begin
    result = db.sigil.session_create(schema_name, user_id, "s3cret123!")
    check("session_create", !result.nil? && !result.access_token.nil? && !result.access_token.empty?)
  rescue StandardError => e
    check("session_create", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
