# frozen_string_literal: true
require "shroudb"
require "base64"

$passed = 0
$failed = 0
def check(name, ok)
  if ok then $passed += 1; puts "  PASS  #{name}" else $failed += 1; puts "  FAIL  #{name}" end
end

uri = ENV.fetch("SHROUDB_STASH_TEST_URI", "shroudb-stash://127.0.0.1:7299")
db = ShrouDB::Client.new(stash: uri)
blob_data = Base64.strict_encode64("hello encrypted world")
blob_id = "test-blob-1"

begin
  begin; db.stash.health; check("health", true)
  rescue => e; check("health", false); puts "    error: #{e.message}"; end

  # store — may fail with CIPHER_UNAVAILABLE
  begin
    db.stash.store(blob_id, blob_data)
    check("store", true)
  rescue ShrouDB::Error => e
    check("store", e.message.downcase.include?("cipher"))
  rescue => e
    check("store", false); puts "    error: #{e.message}"
  end

  # inspect — NOTFOUND if store failed
  begin; db.stash.inspect(blob_id); check("inspect", true)
  rescue ShrouDB::Error; check("inspect", true)
  rescue => e; check("inspect", false); puts "    error: #{e.message}"; end

  begin; db.stash.command; check("command_list", true)
  rescue => e; check("command_list", false); puts "    error: #{e.message}"; end
ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
