#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CODEGEN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SHROUDB_DIR="$(cd "$SCRIPT_DIR/../../shroudb" 2>/dev/null && pwd || echo "")"
TRANSIT_DIR="$(cd "$SCRIPT_DIR/../../shroudb-transit" 2>/dev/null && pwd || echo "")"
AUTH_DIR="$(cd "$SCRIPT_DIR/../../shroudb-auth" 2>/dev/null && pwd || echo "")"
LANG_FILTER=""
KEEP_SERVER=false
MASTER_KEY="$(printf 'a%.0s' {1..64})"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --lang)    LANG_FILTER="$2"; shift 2 ;;
    --keep)    KEEP_SERVER=true; shift ;;
    *)         echo "Usage: $0 [--lang <lang>] [--keep]"; exit 1 ;;
  esac
done

# ── Locate specs ────────────────────────────────────────────────────────────

SHROUDB_SPEC=""
TRANSIT_SPEC=""
AUTH_SPEC=""

if [[ -n "$SHROUDB_DIR" && -f "$SHROUDB_DIR/protocol.toml" ]]; then
  SHROUDB_SPEC="$SHROUDB_DIR/protocol.toml"
fi
if [[ -n "$TRANSIT_DIR" && -f "$TRANSIT_DIR/protocol.toml" ]]; then
  TRANSIT_SPEC="$TRANSIT_DIR/protocol.toml"
fi
if [[ -n "$AUTH_DIR" && -f "$AUTH_DIR/protocol.toml" ]]; then
  AUTH_SPEC="$AUTH_DIR/protocol.toml"
fi

if [[ -z "$SHROUDB_SPEC" && -z "$TRANSIT_SPEC" && -z "$AUTH_SPEC" ]]; then
  echo "ERROR: No protocol.toml found in ../../shroudb/, ../../shroudb-transit/, or ../../shroudb-auth/"
  exit 1
fi

# ── Locate binaries ────────────────────────────────────────────────────────

find_or_build_binary() {
  local name="$1" dir="$2" pkg="${3:-}"
  local bin=""
  if command -v "$name" &>/dev/null; then
    bin="$(command -v "$name")"
  elif [[ -n "$dir" ]]; then
    for candidate in "$dir/target/release/$name" "$dir/target/debug/$name"; do
      if [[ -x "$candidate" ]]; then bin="$candidate"; break; fi
    done
    if [[ -z "$bin" ]]; then
      echo "  Building $name..."
      (cd "$dir" && cargo build ${pkg:+-p $pkg} --release 2>&1 | tail -1)
      bin="$dir/target/release/$name"
    fi
  fi
  echo "$bin"
}

SHROUDB_BIN=""
TRANSIT_BIN=""
AUTH_BIN=""

if [[ -n "$SHROUDB_SPEC" ]]; then
  SHROUDB_BIN=$(find_or_build_binary shroudb "$SHROUDB_DIR" "")
  [[ -z "$SHROUDB_BIN" ]] && echo "WARN: shroudb binary not found, skipping wire tests" && SHROUDB_SPEC=""
fi
if [[ -n "$TRANSIT_SPEC" ]]; then
  TRANSIT_BIN=$(find_or_build_binary shroudb-transit "$TRANSIT_DIR" "shroudb-transit-server")
  [[ -z "$TRANSIT_BIN" ]] && echo "WARN: shroudb-transit binary not found, skipping transit tests" && TRANSIT_SPEC=""
fi
if [[ -n "$AUTH_SPEC" ]]; then
  AUTH_BIN=$(find_or_build_binary shroudb-auth "$AUTH_DIR" "")
  [[ -z "$AUTH_BIN" ]] && echo "WARN: shroudb-auth binary not found, skipping auth tests" && AUTH_SPEC=""
fi

# ── Run codegen ─────────────────────────────────────────────────────────────

echo ""
echo "=== Generating clients ==="
cd "$SCRIPT_DIR"
rm -rf generated/

run_codegen() {
  local spec="$1" output="$2"
  cargo run --manifest-path "$CODEGEN_DIR/Cargo.toml" --release -- \
    --spec "$spec" --lang all --output "$output"
}

if [[ -n "$SHROUDB_SPEC" ]]; then run_codegen "$SHROUDB_SPEC" generated/shroudb; fi
if [[ -n "$TRANSIT_SPEC" ]]; then run_codegen "$TRANSIT_SPEC" generated/transit; fi
if [[ -n "$AUTH_SPEC" ]]; then run_codegen "$AUTH_SPEC" generated/auth; fi

echo ""

# ── Start servers ───────────────────────────────────────────────────────────

find_free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()' 2>/dev/null \
    || ruby -e 'require "socket"; s=TCPServer.new("127.0.0.1",0); puts s.addr[1]; s.close' 2>/dev/null \
    || echo "$1"
}

DATA_DIR="$(mktemp -d)"
PIDS=""

start_server() {
  local name="$1" binary="$2" template="$3" port="$4" extra_env="${5:-}"
  local config="$DATA_DIR/${name}-config.toml"
  local data="$DATA_DIR/${name}-data"
  mkdir -p "$data"
  sed -e "s|{{PORT}}|$port|g" -e "s|{{DATA_DIR}}|$data|g" "$template" > "$config"

  eval "$extra_env SHROUDB_MASTER_KEY=\"$MASTER_KEY\" \"$binary\" --config \"$config\" >/dev/null 2>&1 &"
  local pid=$!
  PIDS="$PIDS $pid"

  # Poll until ready
  local ready=false
  for _ in $(seq 1 50); do
    if bash -c "echo > /dev/tcp/127.0.0.1/$port" 2>/dev/null; then
      ready=true; break
    fi
    sleep 0.1
  done
  if [[ "$ready" != "true" ]]; then
    echo "ERROR: $name did not start within 5 seconds on port $port"
    exit 1
  fi
  echo "  $name ready on port $port (PID $pid)"
}

cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  rm -rf "$DATA_DIR"
}

if [[ "$KEEP_SERVER" == "false" ]]; then
  trap cleanup EXIT
fi

echo "=== Starting servers ==="

SHROUDB_PORT=""
TRANSIT_PORT=""
AUTH_PORT=""

if [[ -n "$SHROUDB_SPEC" ]]; then
  SHROUDB_PORT=$(find_free_port 16399)
  start_server shroudb "$SHROUDB_BIN" "$SCRIPT_DIR/test-config.toml" "$SHROUDB_PORT"
  export SHROUDB_TEST_URI="shroudb://127.0.0.1:$SHROUDB_PORT"
fi
if [[ -n "$TRANSIT_SPEC" ]]; then
  TRANSIT_PORT=$(find_free_port 16499)
  start_server transit "$TRANSIT_BIN" "$SCRIPT_DIR/test-transit-config.toml" "$TRANSIT_PORT"
  export SHROUDB_TRANSIT_TEST_URI="shroudb-transit://127.0.0.1:$TRANSIT_PORT"
fi
if [[ -n "$AUTH_SPEC" ]]; then
  AUTH_PORT=$(find_free_port 14001)
  start_server auth "$AUTH_BIN" "$SCRIPT_DIR/test-auth-config.toml" "$AUTH_PORT"
  export SHROUDB_AUTH_TEST_URL="http://127.0.0.1:$AUTH_PORT"
fi

echo ""

# ── Detect runtimes ─────────────────────────────────────────────────────────

HAS_PYTHON=false; HAS_TYPESCRIPT=false; HAS_GO=false; HAS_RUBY=false
if command -v python3 &>/dev/null; then HAS_PYTHON=true; fi
if command -v node &>/dev/null && command -v npx &>/dev/null; then HAS_TYPESCRIPT=true; fi
if command -v go &>/dev/null; then HAS_GO=true; fi
if command -v ruby &>/dev/null; then HAS_RUBY=true; fi

# ── Run tests ───────────────────────────────────────────────────────────────

TOTAL_PASS=0
TOTAL_FAIL=0
TOTAL_SKIP=0
SUMMARY=""

has_runtime() {
  case "$1" in
    python)     $HAS_PYTHON ;; typescript) $HAS_TYPESCRIPT ;;
    go)         $HAS_GO ;;     ruby)       $HAS_RUBY ;;
  esac
}

run_lang_test() {
  local suite="$1" lang="$2" gen_dir="$3" test_file="$4"

  local label="${suite}/${lang}"

  if [[ -n "$LANG_FILTER" && "$lang" != "$LANG_FILTER" ]]; then
    SUMMARY="${SUMMARY}$(printf '  %-24s SKIP (filtered)\n' "$label")\n"
    TOTAL_SKIP=$((TOTAL_SKIP + 1))
    return
  fi

  if ! has_runtime "$lang"; then
    SUMMARY="${SUMMARY}$(printf '  %-24s SKIP (no runtime)\n' "$label")\n"
    TOTAL_SKIP=$((TOTAL_SKIP + 1))
    return
  fi

  if [[ ! -d "$gen_dir" ]]; then
    SUMMARY="${SUMMARY}$(printf '  %-24s SKIP (not generated)\n' "$label")\n"
    TOTAL_SKIP=$((TOTAL_SKIP + 1))
    return
  fi

  echo "=== $label ==="

  local exit_code=0
  case "$lang" in
    python)
      (cd "$gen_dir" && python3 "$test_file") || exit_code=$?
      ;;
    typescript)
      cp "$test_file" "$gen_dir/test.ts"
      (cd "$gen_dir" && npx --yes tsx test.ts) || exit_code=$?
      ;;
    go)
      mkdir -p "$gen_dir/cmd/test"
      cp "$test_file" "$gen_dir/cmd/test/main.go"
      local go_module
      go_module=$(grep '^module ' "$gen_dir/go.mod" | awk '{print $2}')
      if ! grep -q "^replace" "$gen_dir/go.mod"; then
        printf '\nreplace %s => ./\n' "$go_module" >> "$gen_dir/go.mod"
      fi
      (cd "$gen_dir" && go run ./cmd/test/) || exit_code=$?
      ;;
    ruby)
      (cd "$gen_dir" && ruby -I lib "$test_file") || exit_code=$?
      ;;
  esac

  if [[ $exit_code -eq 0 ]]; then
    SUMMARY="${SUMMARY}$(printf '  %-24s PASS\n' "$label")\n"
    TOTAL_PASS=$((TOTAL_PASS + 1))
  else
    SUMMARY="${SUMMARY}$(printf '  %-24s FAIL\n' "$label")\n"
    TOTAL_FAIL=$((TOTAL_FAIL + 1))
  fi
  echo ""
}

# ── Wire protocol: shroudb ──────────────────────────────────────────────────

if [[ -n "$SHROUDB_SPEC" ]]; then
  run_lang_test shroudb python    generated/shroudb/python     "$SCRIPT_DIR/tests/test_python.py"
  run_lang_test shroudb typescript generated/shroudb/typescript "$SCRIPT_DIR/tests/test_typescript.ts"
  run_lang_test shroudb go        generated/shroudb/go          "$SCRIPT_DIR/tests/test_go.go"
  run_lang_test shroudb ruby      generated/shroudb/ruby        "$SCRIPT_DIR/tests/test_ruby.rb"
fi

# ── Wire protocol: shroudb-transit ──────────────────────────────────────────

if [[ -n "$TRANSIT_SPEC" ]]; then
  run_lang_test transit python     generated/transit/python     "$SCRIPT_DIR/tests/test_transit_python.py"
  run_lang_test transit typescript generated/transit/typescript "$SCRIPT_DIR/tests/test_transit_typescript.ts"
  run_lang_test transit go         generated/transit/go         "$SCRIPT_DIR/tests/test_transit_go.go"
  run_lang_test transit ruby       generated/transit/ruby       "$SCRIPT_DIR/tests/test_transit_ruby.rb"
fi

# ── HTTP API: shroudb-auth ──────────────────────────────────────────────────

if [[ -n "$AUTH_SPEC" ]]; then
  run_lang_test auth python        generated/auth/python        "$SCRIPT_DIR/tests/test_auth_python.py"
  run_lang_test auth typescript    generated/auth/typescript    "$SCRIPT_DIR/tests/test_auth_typescript.ts"
  run_lang_test auth go            generated/auth/go            "$SCRIPT_DIR/tests/test_auth_go.go"
  run_lang_test auth ruby          generated/auth/ruby          "$SCRIPT_DIR/tests/test_auth_ruby.rb"
fi

# ── Results ─────────────────────────────────────────────────────────────────

echo "=== Results ==="
printf "$SUMMARY"
echo ""

if [[ "$KEEP_SERVER" == "true" ]]; then
  echo "Servers still running:"
  [[ -n "$SHROUDB_PORT" ]] && echo "  shroudb:  $SHROUDB_TEST_URI"
  [[ -n "$TRANSIT_PORT" ]] && echo "  transit:  $SHROUDB_TRANSIT_TEST_URI"
  [[ -n "$AUTH_PORT" ]] && echo "  auth:     $SHROUDB_AUTH_TEST_URL"
fi

if [[ $TOTAL_FAIL -gt 0 ]]; then
  echo "$TOTAL_FAIL suite(s) FAILED."
  exit 1
fi

echo "All tested suites passed."
