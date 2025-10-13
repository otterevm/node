#!/bin/bash

set -e

echo "Step 1: Running tempo-bench with max-tps..."
cargo run --bin tempo-bench run-max-tps \
  --tps 20000 \
  --target-urls http://localhost:8545 \
  --disable-thread-pinning true \
  --chain-id 1337

echo ""
echo "Step 2: Finding tempo node process..."
TEMPO_PID=$(pgrep -x tempo)

if [ -z "$TEMPO_PID" ]; then
  echo "No tempo process found"
  exit 1
fi

echo "Found tempo process with PID: $TEMPO_PID"

echo ""
echo "Step 3: Killing tempo node..."
kill $TEMPO_PID

echo "Tempo process killed successfully"

echo ""
echo "Step 4: Analyzing logs..."
python3 analyze_log.py
