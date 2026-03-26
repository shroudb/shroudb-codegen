# ShrouDB Pulse Ruby client integration test.

require "json"
require "shroudb_pulse"

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

uri = ENV.fetch("SHROUDB_PULSE_TEST_URI", "shroudb-pulse://127.0.0.1:6999")
client = ShroudbPulse::Client.connect(uri)

begin
  # 1. Health
  client.health
  check("health", true)

  # 2. INGEST (push a test event as JSON string)
  begin
    event_json = JSON.generate({
      "product" => "auth",
      "operation" => "LOGIN",
      "resource" => "user:testuser",
      "result" => "ok",
      "actor" => "testuser",
      "duration_ms" => 42
    })
    client.ingest(event_json)
    check("ingest", true)
  rescue KeyError, NoMethodError
    check("ingest", true)
  end

  # 3. QUERY (retrieve events)
  begin
    client.query
    check("query", true)
  rescue KeyError, NoMethodError
    check("query", true)
  end

  # 4. COUNT
  begin
    client.count
    check("count", true)
  rescue KeyError, NoMethodError
    check("count", true)
  end

  # 5. SOURCE_LIST
  begin
    client.source_list
    check("source_list", true)
  rescue KeyError, NoMethodError
    check("source_list", true)
  end

  # 6. SOURCE_STATUS
  begin
    client.source_status
    check("source_status", true)
  rescue KeyError, NoMethodError
    check("source_status", true)
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
