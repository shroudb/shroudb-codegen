# frozen_string_literal: true

# ShrouDB Scroll unified SDK Ruby integration test.

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

uri = ENV.fetch("SHROUDB_SCROLL_TEST_URI", "shroudb-scroll://127.0.0.1:7200")
db = ShrouDB::Client.new(scroll: uri)

log = "sandbox-log-rb-#{Time.now.to_i % 100_000}"
group = "workers"

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.scroll.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "scroll")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  begin
    db.scroll.health
    check("health", true)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  begin
    pong = db.scroll.ping
    check("ping", !pong.nil?)
  rescue StandardError => e
    check("ping", false)
    puts "    error: #{e.message}"
  end

  first_payload = Base64.strict_encode64("hello scroll")
  begin
    r = db.scroll.append(log, first_payload)
    check("append: first", r.offset == 0)
  rescue StandardError => e
    check("append: first", false)
    puts "    error: #{e.message}"
  end

  begin
    r = db.scroll.append(log, Base64.strict_encode64("second"))
    check("append: second", r.offset == 1)
  rescue StandardError => e
    check("append: second", false)
    puts "    error: #{e.message}"
  end

  begin
    rr = db.scroll.read(log, 0, 10)
    entries = rr.entries || []
    check("read: count", entries.length == 2)
    # Entries come back as raw Hashes (codegen doesn't emit a typed
    # LogEntry) — fall back to [] lookup when the attribute reader is
    # absent.
    first = entries[0]
    payload_b64 = first.respond_to?(:payload_b64) ? first.payload_b64 : first["payload_b64"]
    check("read: payload roundtrip", Base64.strict_decode64(payload_b64) == "hello scroll")
  rescue StandardError => e
    check("read: count", false)
    puts "    error: #{e.message}"
  end

  begin
    db.scroll.create_group(log, group, "earliest")
    check("create_group", true)
  rescue StandardError => e
    check("create_group", false)
    puts "    error: #{e.message}"
  end

  begin
    rg = db.scroll.read_group(log, group, "reader-1", 10)
    entries = rg.entries || []
    check("read_group: count", entries.length == 2)
  rescue StandardError => e
    check("read_group: count", false)
    puts "    error: #{e.message}"
  end

  begin
    db.scroll.ack(log, group, 0)
    db.scroll.ack(log, group, 1)
    check("ack", true)
  rescue StandardError => e
    check("ack", false)
    puts "    error: #{e.message}"
  end

  begin
    info = db.scroll.log_info(log)
    check("log_info: entries_minted", info.entries_minted == 2)
    check("log_info: has group", (info.groups || []).include?(group))
  rescue StandardError => e
    check("log_info", false)
    puts "    error: #{e.message}"
  end

  begin
    info = db.scroll.group_info(log, group)
    check("group_info: cursor", info.last_delivered_offset == 1)
    check("group_info: pending_count", info.pending_count == 0)
  rescue StandardError => e
    check("group_info", false)
    puts "    error: #{e.message}"
  end

  begin
    db.scroll.delete_group(log, group)
    check("delete_group", true)
  rescue StandardError => e
    check("delete_group", false)
    puts "    error: #{e.message}"
  end

  begin
    db.scroll.delete_log(log)
    check("delete_log", true)
  rescue StandardError => e
    check("delete_log", false)
    puts "    error: #{e.message}"
  end
ensure
  db.close if db.respond_to?(:close)
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit($failed.zero? ? 0 : 1)
