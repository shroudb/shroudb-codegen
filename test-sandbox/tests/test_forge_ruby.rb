# frozen_string_literal: true

# ShrouDB Forge unified SDK Ruby integration test.

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

uri = ENV.fetch("SHROUDB_FORGE_TEST_URI", "shroudb-forge://127.0.0.1:6699")
db = ShrouDB::Client.new(forge: uri)

begin
  # 1. Health via ca_list (forge has no RESP3 HEALTH command)
  begin
    result = db.forge.ca_list
    check("health_via_ca_list", !result.nil?)
  rescue StandardError => e
    check("health_via_ca_list", false)
    puts "    error: #{e.message}"
  end

  # 2. CaCreate — exercises the `SUBJECT` keyword-prefix wire path.
  begin
    result = db.forge.ca_create(
      "codegen-new-ca", "ecdsa-p256", "CN=Codegen New CA", ttl_days: 30
    )
    check("ca_create", !result.nil? && result.name == "codegen-new-ca")
  rescue StandardError => e
    check("ca_create", false)
    puts "    error: #{e.message}"
  end

  # 3. CaInfo
  begin
    result = db.forge.ca_info("test-ca")
    check("ca_info", !result.nil?)
  rescue StandardError => e
    check("ca_info", false)
    puts "    error: #{e.message}"
  end

  # 3. CaList
  begin
    result = db.forge.ca_list
    check("ca_list", !result.nil?)
  rescue StandardError => e
    check("ca_list", false)
    puts "    error: #{e.message}"
  end

  # 4. Issue certificate
  serial = nil
  begin
    result = db.forge.issue("test-ca", "CN=test.example.com", "server")
    serial = result.serial if result
    check("issue", !result.nil? && !serial.nil?)
  rescue StandardError => e
    check("issue", false)
    puts "    error: #{e.message}"
  end

  # 5. Inspect (use serial from issue)
  if serial
    begin
      result = db.forge.inspect("test-ca", serial)
      check("inspect", !result.nil? && result.serial == serial)
    rescue StandardError => e
      check("inspect", false)
      puts "    error: #{e.message}"
    end
  else
    check("inspect", false)
    puts "    skipped: no serial from issue"
  end

  # 6. ListCerts
  begin
    result = db.forge.list_certs("test-ca")
    check("list_certs", !result.nil?)
  rescue StandardError => e
    check("list_certs", false)
    puts "    error: #{e.message}"
  end

  # 7. Revoke (use serial from issue)
  if serial
    begin
      result = db.forge.revoke("test-ca", serial)
      check("revoke", !result.nil?)
    rescue StandardError => e
      check("revoke", false)
      puts "    error: #{e.message}"
    end
  else
    check("revoke", false)
    puts "    skipped: no serial from issue"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
