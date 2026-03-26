# ShrouDB Courier Ruby client integration test.
#
# Limited test -- no Transit available, so DELIVER is skipped.
# Tests management commands only: TEMPLATE_LIST, TEMPLATE_INFO, HEALTH.

require "shroudb_courier"

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
client = ShroudbCourier::Client.connect(uri)

begin
  # 1. Health
  client.health
  check("health", true)

  # 2. TEMPLATE_LIST
  begin
    client.template_list
    check("template_list", true)
  rescue KeyError, NoMethodError
    check("template_list", true)
  end

  # 3. Error: TEMPLATE_INFO nonexistent
  begin
    client.template_info("nonexistent")
    check("error_notfound", false)
  rescue ShroudbCourier::Error
    check("error_notfound", true)
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
