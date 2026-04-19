#!/usr/bin/env bash
set -euo pipefail

# ── Test runner for the unified ShrouDB SDK ────────────────────────────────
#
# Generates a single SDK per language from the Moat composite spec, starts
# each engine server individually, then runs per-engine test files that
# exercise the unified SDK's namespaced API (db.cipher.encrypt, etc.).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CODEGEN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MOAT_DIR="$(cd "$SCRIPT_DIR/../../shroudb-moat" 2>/dev/null && pwd || echo "")"
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

# ── Locate Moat spec ──────────────────────────────────────────────────────

MOAT_SPEC=""
if [[ -n "$MOAT_DIR" && -f "$MOAT_DIR/protocol.toml" ]]; then
  MOAT_SPEC="$MOAT_DIR/protocol.toml"
fi

if [[ -z "$MOAT_SPEC" ]]; then
  echo "ERROR: No protocol.toml found at ../shroudb-moat/protocol.toml"
  exit 1
fi

# ── Engine definitions ───────────────────────────────────────────────────
# Each engine: name, directory, binary name, config template

ENGINES="shroudb cipher sigil forge sentry keep courier chronicle veil stash scroll"

engine_dir() {
  local d="$SCRIPT_DIR/../../shroudb"
  [[ "$1" != "shroudb" ]] && d="$SCRIPT_DIR/../../shroudb-$1"
  cd "$d" 2>/dev/null && pwd || echo ""
}

engine_config() {
  [[ "$1" == "shroudb" ]] && echo "$SCRIPT_DIR/test-config.toml" || echo "$SCRIPT_DIR/test-$1-config.toml"
}

engine_bin_name() {
  [[ "$1" == "shroudb" ]] && echo "shroudb" || echo "shroudb-$1"
}

# ── Locate binaries ──────────────────────────────────────────────────────

find_binary() {
  local name="$1" dir="$2"
  if command -v "$name" &>/dev/null; then
    command -v "$name"
    return
  fi
  for candidate in "$dir/target/debug/$name" "$dir/target/release/$name"; do
    if [[ -x "$candidate" ]]; then echo "$candidate"; return; fi
  done
  echo ""
}

AVAILABLE_ENGINES=""
# Store bin paths and ports via eval
for engine in $ENGINES; do
  dir="$(engine_dir "$engine")"
  if [[ -z "$dir" || ! -d "$dir" ]]; then continue; fi
  config="$(engine_config "$engine")"
  if [[ ! -f "$config" ]]; then continue; fi

  bin_name="$(engine_bin_name "$engine")"
  bin=$(find_binary "$bin_name" "$dir")
  if [[ -z "$bin" ]]; then
    echo "WARN: $bin_name binary not found, skipping $engine tests"
    continue
  fi

  eval "BIN_${engine}=\"$bin\""
  eval "CONFIG_${engine}=\"$config\""
  AVAILABLE_ENGINES="$AVAILABLE_ENGINES $engine"
done

AVAILABLE_ENGINES="${AVAILABLE_ENGINES# }"

if [[ -z "$AVAILABLE_ENGINES" ]]; then
  echo "ERROR: No engine binaries found"
  exit 1
fi

echo "Available engines: $AVAILABLE_ENGINES"

# ── Run codegen (single unified SDK per language) ─────────────────────────

echo ""
echo "=== Generating unified SDKs ==="
cd "$SCRIPT_DIR"
rm -rf generated/

cargo run --manifest-path "$CODEGEN_DIR/Cargo.toml" --release -- \
  --spec "$MOAT_SPEC" --lang all --output generated/ \
  --sdk-version "0.0.0-sandbox"

echo ""

# ── Start servers ────────────────────────────────────────────────────────

find_free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()' 2>/dev/null \
    || ruby -e 'require "socket"; s=TCPServer.new("127.0.0.1",0); puts s.addr[1]; s.close' 2>/dev/null \
    || echo "$1"
}

DATA_DIR="$(mktemp -d)"
PIDS=""

start_engine() {
  local name="$1" binary="$2" config="$3" port="$4" http_port="${5:-}"
  local data="$DATA_DIR/${name}-data"
  mkdir -p "$data"

  # Build config file from template.
  local cfg="$DATA_DIR/${name}-config.toml"
  sed -e "s|{{PORT}}|$port|g" -e "s|{{DATA_DIR}}|$data|g" -e "s|{{HTTP_PORT}}|${http_port:-0}|g" -e "s|{{MINIO_PORT}}|${MINIO_PORT:-9000}|g" -e "s|{{CIPHER_PORT}}|${PORT_cipher:-0}|g" -e "s|{{SENTRY_PORT}}|${PORT_sentry:-0}|g" "$config" > "$cfg"

  SHROUDB_MASTER_KEY="$MASTER_KEY" \
    AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-}" \
    AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-}" \
    AWS_REGION="${AWS_REGION:-}" \
    "$binary" --config "$cfg" >"$DATA_DIR/${name}.log" 2>&1 &
  local pid=$!
  PIDS="$PIDS $pid"

  # Poll until TCP port is reachable. Chronicle in particular can take
  # several seconds to rebuild its index on cold start, so the budget
  # needs to be generous enough for realistic dev-machine timings.
  local ready=false
  for _ in $(seq 1 150); do
    if python3 -c "import socket; s=socket.socket(); s.settimeout(0.5); exit(0 if s.connect_ex(('127.0.0.1',$port))==0 else 1)" 2>/dev/null; then
      ready=true; break
    fi
    sleep 0.1
  done
  if [[ "$ready" != "true" ]]; then
    echo "ERROR: $name did not start within 15 seconds on port $port"
    exit 1
  fi
  echo "  $name ready on port $port (PID $pid)"
}

cleanup() {
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  docker rm -f shroudb-test-minio 2>/dev/null || true
  rm -rf "$DATA_DIR"
}

if [[ "$KEEP_SERVER" == "false" ]]; then
  trap cleanup EXIT
fi

echo "=== Starting servers ==="

# Start MinIO if stash is in the available engines and docker is available.
MINIO_PORT=""
if echo "$AVAILABLE_ENGINES" | grep -q "stash" && command -v docker &>/dev/null; then
  MINIO_PORT=$(find_free_port 19000)
  echo "  Starting MinIO on port $MINIO_PORT..."
  docker rm -f shroudb-test-minio 2>/dev/null || true
  docker run -d --name shroudb-test-minio \
    -p "127.0.0.1:${MINIO_PORT}:9000" \
    -e MINIO_ROOT_USER=minioadmin \
    -e MINIO_ROOT_PASSWORD=minioadmin \
    minio/minio server /data >/dev/null 2>&1

  # Wait for MinIO and create test bucket.
  for _ in $(seq 1 30); do
    if python3 -c "import socket; s=socket.socket(); s.settimeout(0.5); exit(0 if s.connect_ex(('127.0.0.1',$MINIO_PORT))==0 else 1)" 2>/dev/null; then
      break
    fi
    sleep 0.2
  done
  # Create the stash-test bucket.
  # Wait a bit for MinIO to fully initialize, then use mc inside the container.
  sleep 1
  docker exec shroudb-test-minio sh -c 'mc alias set local http://localhost:9000 minioadmin minioadmin && mc mb --ignore-existing local/stash-test' 2>/dev/null || true
  echo "  MinIO ready on port $MINIO_PORT"
  export AWS_ACCESS_KEY_ID=minioadmin
  export AWS_SECRET_ACCESS_KEY=minioadmin
  export AWS_REGION=us-east-1
fi

for engine in $AVAILABLE_ENGINES; do
  port=$(find_free_port 16000)
  # Allocate an HTTP port for engines whose server binds an HTTP endpoint
  # alongside RESP3 (sigil, forge, chronicle, veil). Empty for the rest.
  case "$engine" in
    sigil|forge|chronicle|veil) http_port=$(find_free_port 17000) ;;
    *) http_port="" ;;
  esac
  bin_var="BIN_${engine}"
  config_var="CONFIG_${engine}"

  # Scroll requires the remote Cipher keyring it's configured to use
  # (test-scroll-config.toml → "scroll-sandbox") to exist before the
  # first APPEND. Cipher doesn't seed keyrings at startup, so create
  # it here via the cipher CLI now that cipher is running.
  if [[ "$engine" == "scroll" ]]; then
    cipher_cli="$(find_binary shroudb-cipher-cli "$(engine_dir cipher)")"
    if [[ -n "$cipher_cli" && -n "${PORT_cipher:-}" ]]; then
      "$cipher_cli" --addr "127.0.0.1:${PORT_cipher}" \
        KEYRING CREATE scroll-sandbox AES-256-GCM >/dev/null 2>&1 || true
    fi
  fi

  start_engine "$engine" "${!bin_var}" "${!config_var}" "$port" "$http_port"
  eval "PORT_${engine}=$port"

  # Set URI environment variable.
  upper=$(echo "$engine" | tr '[:lower:]' '[:upper:]')
  # Engines whose sandbox config declares [auth] with method = "token"
  # need the token in the URI so the SDK AUTHs on connect. See
  # test-{stash,forge}-config.toml for why those engines require auth.
  token_prefix=""
  case "$engine" in
    stash|forge) token_prefix="sandbox-test@" ;;
  esac
  if [[ "$engine" == "shroudb" ]]; then
    uri="shroudb://${token_prefix}127.0.0.1:$port"
  else
    uri="shroudb-${engine}://${token_prefix}127.0.0.1:$port"
  fi
  export "SHROUDB_${upper}_TEST_URI=$uri"
  eval "URI_${engine}=\"$uri\""
done

echo ""

# ── Detect runtimes ──────────────────────────────────────────────────────

HAS_PYTHON=false; HAS_TYPESCRIPT=false; HAS_GO=false; HAS_RUBY=false
if command -v python3 &>/dev/null; then HAS_PYTHON=true; fi
if command -v node &>/dev/null && command -v npx &>/dev/null; then HAS_TYPESCRIPT=true; fi
if command -v go &>/dev/null; then HAS_GO=true; fi
if command -v ruby &>/dev/null; then HAS_RUBY=true; fi

# ── Run tests ────────────────────────────────────────────────────────────

TOTAL_PASS=0
TOTAL_FAIL=0
TOTAL_SKIP=0
SUMMARY=""
TS_INSTALLED=false

has_runtime() {
  case "$1" in
    python)     $HAS_PYTHON ;; typescript) $HAS_TYPESCRIPT ;;
    go)         $HAS_GO ;;     ruby)       $HAS_RUBY ;;
  esac
}

run_lang_test() {
  local engine="$1" lang="$2" gen_dir="$3" test_file="$4"
  local label="${engine}/${lang}"

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

  if [[ ! -f "$test_file" ]]; then
    SUMMARY="${SUMMARY}$(printf '  %-24s SKIP (no test file)\n' "$label")\n"
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
      if [[ "$TS_INSTALLED" == "false" ]]; then
        (cd "$gen_dir" && npm install --ignore-scripts 2>/dev/null) || true
        TS_INSTALLED=true
      fi
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

# Run tests per engine × language
for engine in $AVAILABLE_ENGINES; do
  for lang_ext in python:.py typescript:.ts go:.go ruby:.rb; do
    lang="${lang_ext%%:*}"
    ext="${lang_ext#*:}"
    test_file="$SCRIPT_DIR/tests/test_${engine}_${lang}${ext}"
    gen_dir="$SCRIPT_DIR/generated/${lang}"
    run_lang_test "$engine" "$lang" "$gen_dir" "$test_file"
  done
done

# ── Results ──────────────────────────────────────────────────────────────

echo "=== Results ==="
printf "$SUMMARY"
echo ""

if [[ "$KEEP_SERVER" == "true" ]]; then
  echo "Servers still running:"
  for engine in $AVAILABLE_ENGINES; do
    uri_var="URI_${engine}"
    echo "  $engine: ${!uri_var}"
  done
fi

if [[ $TOTAL_FAIL -gt 0 ]]; then
  echo "$TOTAL_FAIL suite(s) FAILED."
  exit 1
fi

echo "All tested suites passed."
