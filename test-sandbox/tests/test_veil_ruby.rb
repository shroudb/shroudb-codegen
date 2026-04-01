# frozen_string_literal: true

# ShrouDB Veil unified SDK Ruby integration test.

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

uri = ENV.fetch("SHROUDB_VEIL_TEST_URI", "shroudb-veil://127.0.0.1:6999")
db = ShrouDB::Client.new(veil: uri)

# Use unique index name per run to avoid "already exists"
idx_name = "test-idx-#{Time.now.to_i % 10000}"

begin
  # 1. Health
  begin
    result = db.veil.health
    check("health", !result.nil?)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 2. IndexCreate
  begin
    result = db.veil.index_create(idx_name)
    check("index_create", !result.nil?)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("index_create", ok)
    puts "    error: #{e.message}" unless ok
  end

  # 3. Tokenize (veil expects base64-encoded plaintext)
  begin
    plaintext_b64 = Base64.strict_encode64("hello")
    result = db.veil.tokenize(idx_name, plaintext_b64)
    check("tokenize", !result.nil? && !result.tokens.nil?)
  rescue StandardError => e
    check("tokenize", false)
    puts "    error: #{e.message}"
  end

  # 4. Put (store blind tokens for an entry)
  begin
    result = db.veil.put(idx_name, "entry-1", plaintext_b64)
    check("put", !result.nil?)
  rescue StandardError => e
    check("put", false)
    puts "    error: #{e.message}"
  end

  # 5. Search (search by token)
  begin
    result = db.veil.search(idx_name, plaintext_b64)
    check("search", !result.nil?)
  rescue StandardError => e
    check("search", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
