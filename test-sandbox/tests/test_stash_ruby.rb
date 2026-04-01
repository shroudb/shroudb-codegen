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
blob_id = "test-blob-rb-#{Time.now.to_i % 100000}"

begin
  begin; db.stash.health; check("health", true)
  rescue => e; check("health", false); puts "    error: #{e.message}"; end

  begin; db.stash.store(blob_id, blob_data); check("store", true)
  rescue => e; check("store", false); puts "    error: #{e.message}"; end

  begin; db.stash.inspect(blob_id); check("inspect", true)
  rescue => e; check("inspect", false); puts "    error: #{e.message}"; end

  begin; db.stash.retrieve(blob_id); check("retrieve", true)
  rescue => e; check("retrieve", false); puts "    error: #{e.message}"; end

  begin; db.stash.revoke(blob_id, soft: true); check("revoke_soft", true)
  rescue => e; check("revoke_soft", false); puts "    error: #{e.message}"; end

  begin
    db.stash.retrieve(blob_id)
    check("error_after_revoke", false)
  rescue ShrouDB::Error
    check("error_after_revoke", true)
  rescue => e
    check("error_after_revoke", false); puts "    unexpected: #{e.message}"
  end

  # Hard revoke (crypto-shred)
  blob_id2 = "#{blob_id}-shred"
  begin
    db.stash.store(blob_id2, blob_data)
    db.stash.revoke(blob_id2)
    check("revoke_hard", true)
  rescue => e
    check("revoke_hard", false); puts "    error: #{e.message}"
  end

  begin
    db.stash.retrieve(blob_id2)
    check("error_after_shred", false)
  rescue ShrouDB::Error
    check("error_after_shred", true)
  rescue => e
    check("error_after_shred", false); puts "    unexpected: #{e.message}"
  end
ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
