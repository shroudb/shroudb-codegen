# frozen_string_literal: true

# ShrouDB Chronicle unified SDK Ruby integration test.

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

uri = ENV.fetch("SHROUDB_CHRONICLE_TEST_URI", "shroudb-chronicle://127.0.0.1:6899")
db = ShrouDB::Client.new(chronicle: uri)

begin
  # 1. Health
  begin
    result = db.chronicle.health
    check("health", !result.nil?)
  rescue StandardError => e
    check("health", false)
    puts "    error: #{e.message}"
  end

  # 2. Ingest (push a test event as JSON string)
  begin
    event_json = JSON.generate({
      "id" => "test-event-1",
      "engine" => "shroudb",
      "operation" => "sdk_test",
      "resource" => "test/resource",
      "result" => "ok",
      "actor" => "user:test@example.com",
      "timestamp" => Time.now.to_i,
      "duration_ms" => 1
    })
    result = db.chronicle.ingest(event_json)
    check("ingest", !result.nil?)
  rescue StandardError => e
    check("ingest", false)
    puts "    error: #{e.message}"
  end

  # 3. Query (retrieve events)
  begin
    result = db.chronicle.query
    check("query", !result.nil?)
  rescue StandardError => e
    check("query", false)
    puts "    error: #{e.message}"
  end

  # 4. Count
  begin
    result = db.chronicle.count
    check("count", !result.nil?)
  rescue StandardError => e
    check("count", false)
    puts "    error: #{e.message}"
  end

  # 5. IngestBatch
  begin
    batch = [
        {
          "id" => "batch-event-1",
          "engine" => "shroudb",
          "operation" => "sdk_test_batch",
          "resource" => "test/batch",
          "result" => "ok",
          "actor" => "user:batch@example.com",
          "timestamp" => Time.now.to_i,
          "duration_ms" => 2
        },
        {
          "id" => "batch-event-2",
          "engine" => "shroudb",
          "operation" => "sdk_test_batch",
          "resource" => "test/batch",
          "result" => "ok",
          "actor" => "user:batch@example.com",
          "timestamp" => Time.now.to_i,
          "duration_ms" => 3
        }
      ]
    result = db.chronicle.ingest_batch(batch)
    check("ingest_batch", !result.nil?)
  rescue StandardError => e
    check("ingest_batch", false)
    puts "    error: #{e.message}"
  end

  # 6. Actors
  begin
    result = db.chronicle.actors
    check("actors", !result.nil?)
  rescue StandardError => e
    check("actors", false)
    puts "    error: #{e.message}"
  end

  # 7. Hotspots
  begin
    result = db.chronicle.hotspots
    check("hotspots", !result.nil?)
  rescue StandardError => e
    check("hotspots", false)
    puts "    error: #{e.message}"
  end

ensure
  db.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
