# frozen_string_literal: true

# ShrouDB Veil unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"
require "base64"
require "openssl"
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

  # ── Blind (E2EE) operations ──────────────────────────────────

  client_key = ("\x42" * 32).b

  blind_tokens_fn = lambda do |text|
    words = text.downcase.split(/[^a-z0-9]+/).reject(&:empty?)
    word_tokens = words.map { |w| "w:#{w}" }.uniq.sort
    trigram_tokens = words.flat_map { |w|
      next [] if w.length < 3
      (0..w.length - 3).map { |i| "t:#{w[i, 3]}" }
    }.uniq.sort

    do_hmac = lambda { |token| OpenSSL::HMAC.hexdigest("SHA256", client_key, token) }

    token_set = {
      words: word_tokens.map { |t| do_hmac.call(t) },
      trigrams: trigram_tokens.map { |t| do_hmac.call(t) }
    }
    Base64.strict_encode64(JSON.generate(token_set))
  end

  # put ... blind
  begin
    tokens_b64 = blind_tokens_fn.call("hello world")
    result = db.veil.put(idx_name, "blind-1", tokens_b64, blind: true)
    check("put_blind", !result.nil?)
  rescue StandardError => e
    check("put_blind", false)
    puts "    error: #{e.message}"
  end

  # search ... blind (exact)
  begin
    query_b64 = blind_tokens_fn.call("hello")
    result = db.veil.search(idx_name, query_b64, mode: "exact", blind: true)
    check("search_blind", !result.nil?)
  rescue StandardError => e
    check("search_blind", false)
    puts "    error: #{e.message}"
  end

  # search ... blind with limit
  begin
    query_b64 = blind_tokens_fn.call("hello")
    result = db.veil.search(idx_name, query_b64, mode: "contains", limit: 5, blind: true)
    check("search_blind_with_limit", !result.nil?)
  rescue StandardError => e
    check("search_blind_with_limit", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
