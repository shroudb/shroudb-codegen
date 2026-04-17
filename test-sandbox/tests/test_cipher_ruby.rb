# frozen_string_literal: true

# ShrouDB Cipher unified SDK Ruby integration test.

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

uri = ENV.fetch("SHROUDB_CIPHER_TEST_URI", "shroudb-cipher://127.0.0.1:6599")
db = ShrouDB::Client.new(cipher: uri)

plaintext_b64 = Base64.strict_encode64("hello world")
data_b64 = Base64.strict_encode64("sign this message")

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.cipher.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "cipher")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  # 1. Health
  begin
    db.cipher.health
    check("health", true)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 2. Rotate AES keyring
  begin
    db.cipher.rotate("test-aes", force: true)
    check("rotate_aes", true)
  rescue StandardError => e
    check("rotate_aes", false)
    puts "    error: #{e.message}"
  end

  # 3. Encrypt
  ciphertext = nil
  begin
    enc = db.cipher.encrypt("test-aes", plaintext_b64)
    ciphertext = enc["ciphertext"]
    check("encrypt", !ciphertext.nil? && !ciphertext.empty?)
  rescue StandardError => e
    check("encrypt", false)
    puts "    error: #{e.message}"
  end

  # 4. Decrypt
  if ciphertext
    begin
      dec = db.cipher.decrypt("test-aes", ciphertext)
      check("decrypt", dec["plaintext"] == plaintext_b64)
    rescue StandardError => e
      check("decrypt", false)
      puts "    error: #{e.message}"
    end
  else
    check("decrypt", false)
    puts "    skipped: no ciphertext from encrypt"
  end

  # 5. Rewrap
  if ciphertext
    begin
      rew = db.cipher.rewrap("test-aes", ciphertext)
      new_ct = rew["ciphertext"]
      check("rewrap", !new_ct.nil? && new_ct != ciphertext)
    rescue StandardError => e
      check("rewrap", false)
      puts "    error: #{e.message}"
    end
  else
    check("rewrap", false)
    puts "    skipped: no ciphertext from encrypt"
  end

  # 6. Rotate ed25519 keyring
  begin
    db.cipher.rotate("test-ed25519", force: true)
    check("rotate_ed25519", true)
  rescue StandardError => e
    check("rotate_ed25519", false)
    puts "    error: #{e.message}"
  end

  # 7. Sign
  signature = nil
  begin
    sig = db.cipher.sign("test-ed25519", data_b64)
    signature = sig["signature"]
    check("sign", !signature.nil? && !signature.empty?)
  rescue StandardError => e
    check("sign", false)
    puts "    error: #{e.message}"
  end

  # 8. Verify signature
  if signature
    begin
      ver = db.cipher.verify_signature("test-ed25519", data_b64, signature)
      valid = ver["valid"]
      check("verify_signature", valid == true || valid == "true")
    rescue StandardError => e
      check("verify_signature", false)
      puts "    error: #{e.message}"
    end
  else
    check("verify_signature", false)
    puts "    skipped: no signature from sign"
  end

  # 9. Generate data key
  begin
    result = db.cipher.generate_data_key("test-aes")
    check("generate_data_key", !result.nil? && !result["plaintext_key"].nil? && !result["wrapped_key"].nil?)
  rescue StandardError => e
    check("generate_data_key", false)
    puts "    error: #{e.message}"
  end

  # 10. Key info
  begin
    result = db.cipher.key_info("test-aes")
    check("key_info", !result.nil? && result["keyring"] == "test-aes")
  rescue StandardError => e
    check("key_info", false)
    puts "    error: #{e.message}"
  end

  # 11. Error: NOTFOUND
  begin
    db.cipher.encrypt("nonexistent-keyring-xyz", plaintext_b64)
    check("error_notfound", false)
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
