#!/bin/bash

# Script to easily switch between different reth versions for testing

set -e

FEATURE_COMMIT="1619408"
MAIN_COMMIT="d2070f4de34f523f6097ebc64fa9d63a04878055"

print_usage() {
    echo "Usage: ./switch_reth.sh [feature|main]"
    echo ""
    echo "Options:"
    echo "  feature  - Switch to commit 1619408 (feature commit)"
    echo "  main     - Switch to commit d2070f4 (main reth commit)"
    echo ""
    echo "After switching, run: cargo update -p reth && cargo build"
}

if [ $# -eq 0 ]; then
    print_usage
    exit 1
fi

case "$1" in
    feature)
        echo "Switching to feature commit $FEATURE_COMMIT..."
        sed -i '' 's/git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "[^"]*"/git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "'$FEATURE_COMMIT'"/g' Cargo.toml
        echo "✓ Switched to feature commit $FEATURE_COMMIT"
        ;;
    main)
        echo "Switching to main commit $MAIN_COMMIT..."
        sed -i '' 's/git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "[^"]*"/git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "'$MAIN_COMMIT'"/g' Cargo.toml
        echo "✓ Switched to main commit $MAIN_COMMIT"
        ;;
    *)
        echo "Error: Unknown option '$1'"
        print_usage
        exit 1
        ;;
esac

echo ""
echo "Updating dependencies..."
cargo update -p reth

echo ""
echo "Building tempo with new reth version..."
cargo build --release

echo ""
echo "✓ Build complete! Tempo is now ready to run with the new reth version."
echo ""
echo "Starting tempo node..."
echo ""

tempo node \
  --http \
  --http.addr 0.0.0.0 \
  --http.port 8545 \
  --http.api all \
  --datadir ./data \
  --dev \
  --dev.block-time 1s \
  --chain genesis.json \
  --engine.disable-precompile-cache \
  --builder.gaslimit 3000000000 \
  --builder.max-tasks 8 \
  --builder.deadline 4 \
  --txpool.pending-max-count 10000000000000 \
  --txpool.basefee-max-count 10000000000000 \
  --txpool.queued-max-count 10000000000000 \
  --txpool.pending-max-size 10000 \
  --txpool.basefee-max-size 10000 \
  --txpool.queued-max-size 10000 \
  --txpool.max-new-pending-txs-notifications 10000000 \
  --txpool.max-account-slots 500000 \
  --txpool.max-pending-txns 10000000000000 \
  --txpool.max-new-txns 10000000000000 \
  --txpool.disable-transactions-backup \
  --txpool.additional-validation-tasks 8 \
  --txpool.minimal-protocol-fee 0 \
  --txpool.minimum-priority-fee 0 \
  --rpc.max-connections 429496729 \
  --rpc.max-request-size 1000000 \
  --rpc.max-response-size 1000000 \
  --max-tx-reqs 1000000 2>&1 | tee >(rg "build_payload|Received block from consensus engine|State root task finished|Block added to canonical chain" > debug.log)
