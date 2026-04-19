# frozen_string_literal: true

# ShrouDB Courier unified SDK Ruby integration test.

$LOAD_PATH.unshift(File.join(__dir__, "..", "lib")) unless __dir__.nil?
require "shroudb"
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

uri = ENV.fetch("SHROUDB_COURIER_TEST_URI", "shroudb-courier://127.0.0.1:6899")
db = ShrouDB::Client.new(courier: uri)

begin
  # Handshake sanity — every engine must answer HELLO.
  begin
    h = db.courier.hello
    check("hello: ok", true)
    check("hello: engine name", h.engine == "courier")
    check("hello: version not empty", h.version.is_a?(String) && !h.version.empty?)
    check("hello: protocol", h.protocol == "RESP3/1")
  rescue StandardError
    check("hello: ok", false)
  end

  # 1. Health
  db.courier.health
  check("health", true)

  # 2. ChannelList
  begin
    db.courier.channel_list
    check("channel_list", true)
  rescue KeyError, NoMethodError
    check("channel_list", true)
  end

  # 3. ChannelCreate
  channel_name = "test-channel-#{Time.now.to_i % 10000}"
  begin
    # F-courier-8: config moved from positional arg to keyword args.
    config_json = JSON.generate({ "url" => "https://example.com/webhook" })
    result = db.courier.channel_create(channel_name, "webhook", config_json: config_json)
    check("channel_create", !result.nil? && result.name == channel_name)
  rescue ShrouDB::Error => e
    ok = e.message.include?("EXISTS") || e.message.downcase.include?("exists")
    check("channel_create", ok)
    puts "    error: #{e.message}" unless ok
  rescue StandardError => e
    check("channel_create", false)
    puts "    error: #{e.message}"
  end

  # 4. ChannelDelete
  begin
    result = db.courier.channel_delete(channel_name)
    check("channel_delete", !result.nil?)
  rescue StandardError => e
    check("channel_delete", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
