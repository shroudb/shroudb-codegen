#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CODEGEN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SHROUDB_DIR="$(cd "$SCRIPT_DIR/../../shroudb" 2>/dev/null && pwd || echo "")"
SPEC=""
LANG_FILTER=""
KEEP_SERVER=false
MASTER_KEY="$(printf 'a%.0s' {1..64})"

# ── Parse args ──────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec)    SPEC="$2"; shift 2 ;;
    --lang)    LANG_FILTER="$2"; shift 2 ;;
    --keep)    KEEP_SERVER=true; shift ;;
    *)         echo "Usage: $0 [--spec <path>] [--lang <lang>] [--keep]"; exit 1 ;;
  esac
done

# ── Locate protocol.toml ───────────────────────────────────────────────────

if [[ -z "$SPEC" ]]; then
  if [[ -n "$SHROUDB_DIR" && -f "$SHROUDB_DIR/protocol.toml" ]]; then
    SPEC="$SHROUDB_DIR/protocol.toml"
  else
    echo "ERROR: Cannot find protocol.toml."
    echo "  Expected at: ../../shroudb/protocol.toml"
    echo "  Override with: --spec <path>"
    exit 1
  fi
fi

if [[ ! -f "$SPEC" ]]; then
  echo "ERROR: Spec file not found: $SPEC"
  exit 1
fi

echo "=== Protocol spec: $SPEC ==="

# ── Locate shroudb binary ──────────────────────────────────────────────────

SHROUDB_BIN=""
USE_DOCKER=false
SHROUDB_IMAGE="ghcr.io/shroudb/shroudb:latest"

if command -v shroudb &>/dev/null; then
  SHROUDB_BIN="$(command -v shroudb)"
elif [[ -n "$SHROUDB_DIR" ]]; then
  # Try pre-built binary
  for candidate in \
    "$SHROUDB_DIR/target/release/shroudb" \
    "$SHROUDB_DIR/target/debug/shroudb"; do
    if [[ -x "$candidate" ]]; then
      SHROUDB_BIN="$candidate"
      break
    fi
  done

  # Build if needed
  if [[ -z "$SHROUDB_BIN" ]]; then
    echo "=== Building shroudb server ==="
    (cd "$SHROUDB_DIR" && cargo build -p shroudb-server --release 2>&1 | tail -3)
    SHROUDB_BIN="$SHROUDB_DIR/target/release/shroudb"
  fi
fi

# Fall back to Docker if no local binary
if [[ -z "$SHROUDB_BIN" || ! -x "$SHROUDB_BIN" ]]; then
  if command -v docker &>/dev/null; then
    echo "=== No local binary found, using Docker image: $SHROUDB_IMAGE ==="
    USE_DOCKER=true
  else
    echo "ERROR: Cannot find shroudb binary or Docker."
    echo "  Install shroudb on \$PATH, ensure ../../shroudb/ exists, or install Docker."
    exit 1
  fi
else
  echo "=== Using shroudb: $SHROUDB_BIN ==="
fi

# ── Run codegen ─────────────────────────────────────────────────────────────

echo ""
echo "=== Generating clients ==="

cd "$SCRIPT_DIR"
rm -rf generated/

cargo run --manifest-path "$CODEGEN_DIR/Cargo.toml" --release -- \
  --spec "$SPEC" --lang all --output generated/

echo ""

# ── Start server ────────────────────────────────────────────────────────────

# Find a free port
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()' 2>/dev/null \
  || ruby -e 'require "socket"; s=TCPServer.new("127.0.0.1",0); puts s.addr[1]; s.close' 2>/dev/null \
  || echo "16399")

DATA_DIR="$(mktemp -d)"
CONFIG_FILE="$DATA_DIR/config.toml"
CONTAINER_ID=""
SERVER_PID=""

# Write config from template
sed -e "s|{{PORT}}|$PORT|g" -e "s|{{DATA_DIR}}|$DATA_DIR|g" \
  "$SCRIPT_DIR/test-config.toml" > "$CONFIG_FILE"

echo "=== Starting ShrouDB server (port $PORT) ==="

if [[ "$USE_DOCKER" == "true" ]]; then
  # Docker needs internal bind on 0.0.0.0:6399, port-mapped to host
  DOCKER_CONFIG="$DATA_DIR/docker-config.toml"
  sed -e "s|{{PORT}}|6399|g" -e "s|{{DATA_DIR}}|/data|g" -e "s|127.0.0.1|0.0.0.0|g" \
    "$SCRIPT_DIR/test-config.toml" > "$DOCKER_CONFIG"

  CONTAINER_ID=$(docker run -d --rm \
    -p "127.0.0.1:$PORT:6399" \
    -e "SHROUDB_MASTER_KEY=$MASTER_KEY" \
    -e "LOG_LEVEL=warn" \
    -v "$DOCKER_CONFIG:/config.toml:ro" \
    --tmpfs /data \
    "$SHROUDB_IMAGE" \
    --config /config.toml)
else
  SHROUDB_MASTER_KEY="$MASTER_KEY" "$SHROUDB_BIN" --config "$CONFIG_FILE" &
  SERVER_PID=$!
fi

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  if [[ -n "${CONTAINER_ID:-}" ]]; then
    docker stop "$CONTAINER_ID" >/dev/null 2>&1 || true
  fi
  rm -rf "$DATA_DIR"
}

if [[ "$KEEP_SERVER" == "false" ]]; then
  trap cleanup EXIT
fi

# Poll until ready (5s timeout)
READY=false
for i in $(seq 1 50); do
  if bash -c "echo > /dev/tcp/127.0.0.1/$PORT" 2>/dev/null; then
    READY=true
    break
  fi
  sleep 0.1
done

if [[ "$READY" != "true" ]]; then
  echo "ERROR: Server did not start within 5 seconds on port $PORT"
  exit 1
fi

if [[ "$USE_DOCKER" == "true" ]]; then
  echo "Server ready (container ${CONTAINER_ID:0:12})."
else
  echo "Server ready (PID $SERVER_PID)."
fi
echo ""

export SHROUDB_TEST_URI="shroudb://127.0.0.1:$PORT"

# ── Detect runtimes ─────────────────────────────────────────────────────────

declare -A RUNTIMES
RUNTIMES[python]=""
RUNTIMES[typescript]=""
RUNTIMES[go]=""
RUNTIMES[ruby]=""

if command -v python3 &>/dev/null; then RUNTIMES[python]="python3"; fi
if command -v node &>/dev/null && command -v npx &>/dev/null; then RUNTIMES[typescript]="npx"; fi
if command -v go &>/dev/null; then RUNTIMES[go]="go"; fi
if command -v ruby &>/dev/null; then RUNTIMES[ruby]="ruby"; fi

# ── Run tests ───────────────────────────────────────────────────────────────

declare -A RESULTS
TOTAL_PASS=0
TOTAL_FAIL=0
TOTAL_SKIP=0

run_test() {
  local lang="$1"
  local runtime="${RUNTIMES[$lang]}"

  if [[ -n "$LANG_FILTER" && "$lang" != "$LANG_FILTER" ]]; then
    RESULTS[$lang]="SKIP (filtered)"
    TOTAL_SKIP=$((TOTAL_SKIP + 1))
    return
  fi

  if [[ -z "$runtime" ]]; then
    echo "=== $lang === SKIP (runtime not found)"
    echo ""
    RESULTS[$lang]="SKIP (no runtime)"
    TOTAL_SKIP=$((TOTAL_SKIP + 1))
    return
  fi

  echo "=== $lang ==="

  local exit_code=0
  case "$lang" in
    python)
      (cd generated/python && python3 "$SCRIPT_DIR/tests/test_python.py") || exit_code=$?
      ;;
    typescript)
      # Install tsx if needed, then run
      if ! npx tsx --version &>/dev/null 2>&1; then
        (cd generated/typescript && npm install --save-dev tsx typescript 2>&1 | tail -1)
      fi
      (cd generated/typescript && npx tsx "$SCRIPT_DIR/tests/test_typescript.ts") || exit_code=$?
      ;;
    go)
      # Set up test binary
      mkdir -p generated/go/cmd/test
      cp "$SCRIPT_DIR/tests/test_go.go" generated/go/cmd/test/main.go
      # Add replace directive so the local module resolves
      local go_module
      go_module=$(grep '^module ' generated/go/go.mod | awk '{print $2}')
      if ! grep -q "^replace" generated/go/go.mod; then
        echo "" >> generated/go/go.mod
        echo "replace $go_module => ./" >> generated/go/go.mod
      fi
      (cd generated/go && go run ./cmd/test/) || exit_code=$?
      ;;
    ruby)
      (cd generated/ruby && ruby -I lib "$SCRIPT_DIR/tests/test_ruby.rb") || exit_code=$?
      ;;
  esac

  if [[ $exit_code -eq 0 ]]; then
    RESULTS[$lang]="PASS"
    TOTAL_PASS=$((TOTAL_PASS + 1))
  else
    RESULTS[$lang]="FAIL"
    TOTAL_FAIL=$((TOTAL_FAIL + 1))
  fi
  echo ""
}

run_test python
run_test typescript
run_test go
run_test ruby

# ── Results ─────────────────────────────────────────────────────────────────

echo "=== Results ==="
printf "  %-14s %s\n" "Python" "${RESULTS[python]}"
printf "  %-14s %s\n" "TypeScript" "${RESULTS[typescript]}"
printf "  %-14s %s\n" "Go" "${RESULTS[go]}"
printf "  %-14s %s\n" "Ruby" "${RESULTS[ruby]}"
echo ""

if [[ "$KEEP_SERVER" == "true" ]]; then
  echo "Server still running on $SHROUDB_TEST_URI (PID $SERVER_PID)"
fi

if [[ $TOTAL_FAIL -gt 0 ]]; then
  echo "$TOTAL_FAIL language(s) FAILED."
  exit 1
fi

echo "All tested languages passed."
