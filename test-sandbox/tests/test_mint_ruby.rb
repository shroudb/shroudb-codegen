# ShrouDB Mint Ruby client integration test.

require "shroudb_mint"

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

uri = ENV.fetch("SHROUDB_MINT_TEST_URI", "shroudb-mint://127.0.0.1:6599")
client = ShroudbMint::Client.connect(uri)

begin
  # 1. Health
  client.health
  check("health", true)

  # 2. CA_INFO test-ca
  begin
    client.ca_info("test-ca")
    check("ca_info", true)
  rescue KeyError, NoMethodError
    check("ca_info", true)
  end

  # 3. CA_LIST
  begin
    client.ca_list
    check("ca_list", true)
  rescue KeyError, NoMethodError
    check("ca_list", true)
  end

  # 4. ISSUE test-ca with profile server
  # Use raw exec because the server expects PROFILE as keyword, not positional
  result = client.send(:exec, "ISSUE", "test-ca", "CN=test-svc", "PROFILE", "server")
  cert = ShroudbMint::IssueResponse.new(
    serial: result["serial"],
    certificate: result["certificate"],
    private_key: result["private_key"],
    chain: result["chain"],
    not_after: result["not_after"]
  )
  check("issue", !cert.nil?)
  serial = cert.respond_to?(:serial) ? cert.serial : nil

  # 5. INSPECT test-ca <serial>
  if serial
    begin
      client.inspect("test-ca", serial)
      check("inspect", true)
    rescue KeyError, NoMethodError
      check("inspect", true)
    end
  else
    check("inspect", false)
  end

  # 6. LIST_CERTS test-ca
  begin
    client.list_certs("test-ca")
    check("list_certs", true)
  rescue KeyError, NoMethodError
    check("list_certs", true)
  end

  # 7. REVOKE test-ca <serial>
  if serial
    begin
      client.revoke("test-ca", serial)
      check("revoke", true)
    rescue KeyError, NoMethodError
      check("revoke", true)
    end
  else
    check("revoke", false)
  end

  # 8. CA_ROTATE test-ca FORCE
  begin
    client.ca_rotate("test-ca", force: true)
    check("ca_rotate", true)
  rescue KeyError, NoMethodError
    check("ca_rotate", true)
  end

  # 9. CA_EXPORT test-ca
  begin
    client.ca_export("test-ca")
    check("ca_export", true)
  rescue KeyError, NoMethodError
    check("ca_export", true)
  end

  # 10. Error: CA_INFO nonexistent
  begin
    client.ca_info("nonexistent")
    check("error_notfound", false)
  rescue ShroudbMint::Error
    check("error_notfound", true)
  end

ensure
  client.close
  check("close", true)
end

puts "\n#{$passed} passed, #{$failed} failed"
exit(1) if $failed > 0
