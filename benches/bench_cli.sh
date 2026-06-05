#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== 1. Refs: symlens vs grep vs rg ==="
hyperfine -N --warmup 3 \
    'symlens refs CallGraph' \
    'rg -n --type rust "CallGraph"'

echo ""
echo "=== 2. Search: symlens vs grep vs rg ==="
hyperfine -N --warmup 3 \
    'symlens search CallGraph' \
    'rg -n --type rust "fn CallGraph|struct CallGraph"'

echo ""
echo "=== 3. Callers (symlens only) ==="
hyperfine -N --warmup 3 'symlens callers run'

echo ""
echo "=== 4. Impact (symlens only) ==="
hyperfine -N --warmup 3 'symlens graph impact run'

echo ""
echo "=== 5. Daemon vs CLI: search ==="
echo "Starting daemon in background..."
symlens watch --serve &
DAEMON_PID=$!
sleep 1

hyperfine -N --warmup 3 \
    'symlens search CallGraph' \
    'symlens --daemon search CallGraph'

echo ""
echo "=== 6. Daemon vs CLI: refs ==="
hyperfine -N --warmup 3 \
    'symlens refs CallGraph' \
    'symlens --daemon refs CallGraph'

echo ""
echo "=== 7. Daemon vs CLI: callers ==="
hyperfine -N --warmup 3 \
    'symlens callers run' \
    'symlens --daemon callers run'

echo ""
echo "=== 8. Daemon vs CLI: impact ==="
hyperfine -N --warmup 3 \
    'symlens graph impact run' \
    'symlens --daemon graph impact run'

kill $DAEMON_PID 2>/dev/null || true
wait $DAEMON_PID 2>/dev/null || true
echo "Daemon stopped."
