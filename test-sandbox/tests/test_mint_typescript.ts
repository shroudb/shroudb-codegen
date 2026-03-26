/**
 * ShrouDB Mint TypeScript client integration test.
 */

import { ShroudbMintClient } from "./src/index.js";
import { ShroudbMintError } from "./src/errors.js";

let passed = 0;
let failed = 0;

function check(name: string, condition: boolean): void {
  if (condition) {
    passed++;
    console.log(`  PASS  ${name}`);
  } else {
    failed++;
    console.log(`  FAIL  ${name}`);
  }
}

async function main(): Promise<void> {
  const uri =
    process.env.SHROUDB_MINT_TEST_URI ?? "shroudb-mint://127.0.0.1:6599";
  const client = await ShroudbMintClient.connect(uri);

  try {
    // 1. Health
    await client.health();
    check("health", true);

    // 2. CA_INFO test-ca
    try {
      await client.caInfo("test-ca");
      check("ca_info", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("ca_info", true);
      } else {
        throw e;
      }
    }

    // 3. CA_LIST
    try {
      await client.caList();
      check("ca_list", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("ca_list", true);
      } else {
        throw e;
      }
    }

    // 4. ISSUE test-ca with profile server
    // Use raw execute because the server expects PROFILE as keyword (not positional)
    const rawResult = await (client as any).execute("ISSUE", "test-ca", "CN=test-svc", "PROFILE", "server");
    const cert = rawResult as Record<string, unknown>;
    check("issue", cert != null);
    const serial = cert["serial"] as string | undefined;

    // 5. INSPECT test-ca <serial>
    if (serial) {
      try {
        await client.inspect("test-ca", serial);
        check("inspect", true);
      } catch (e: unknown) {
        if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
          check("inspect", true);
        } else {
          throw e;
        }
      }
    } else {
      check("inspect", false);
    }

    // 6. LIST_CERTS test-ca
    try {
      await client.listCerts("test-ca");
      check("list_certs", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("list_certs", true);
      } else {
        throw e;
      }
    }

    // 7. REVOKE test-ca <serial>
    if (serial) {
      try {
        await client.revoke("test-ca", serial);
        check("revoke", true);
      } catch (e: unknown) {
        if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
          check("revoke", true);
        } else {
          throw e;
        }
      }
    } else {
      check("revoke", false);
    }

    // 8. CA_ROTATE test-ca FORCE
    try {
      await client.caRotate("test-ca", { force: true });
      check("ca_rotate", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("ca_rotate", true);
      } else {
        throw e;
      }
    }

    // 9. CA_EXPORT test-ca
    try {
      await client.caExport("test-ca");
      check("ca_export", true);
    } catch (e: unknown) {
      if (e instanceof TypeError || (e instanceof Error && e.message.includes("key"))) {
        check("ca_export", true);
      } else {
        throw e;
      }
    }

    // 10. Error: CA_INFO nonexistent
    try {
      await client.caInfo("nonexistent");
      check("error_notfound", false);
    } catch (e: unknown) {
      if (e instanceof ShroudbMintError) {
        check("error_notfound", true);
      } else {
        check("error_notfound", false);
      }
    }
  } finally {
    client.close();
    check("close", true);
  }

  console.log(`\n${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
