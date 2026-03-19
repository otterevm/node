#!/bin/bash
set -e

echo "[1/5] Fetching upstream..."
git fetch upstream || {
    echo "Setting up upstream remote..."
    git remote add upstream https://github.com/tempoxyz/tempo.git
    git fetch upstream
}

echo "[2/5] Updating main..."
git checkout main
git merge upstream/main --no-edit || {
    echo "Merge conflicts in main. Resolve manually."
    exit 1
}
git push origin main

echo "[3/5] Updating otterevm..."
git checkout otterevm
git merge main --no-edit || {
    echo "Merge conflicts detected. Resolving common conflicts..."

    # For branding files, keep ours (OtterEVM)
    git checkout --ours README.md 2>/dev/null || echo "README.md not in conflict"
    
    # For currency identifier (we use FEE, upstream uses USD), keep ours
    git checkout --ours crates/precompiles/src/tip20/mod.rs 2>/dev/null || echo "tip20/mod.rs not in conflict"
    git checkout --ours crates/revm/src/common.rs 2>/dev/null || echo "revm/src/common.rs not in conflict"
    git checkout --ours xtask/src/genesis_args.rs 2>/dev/null || echo "xtask/src/genesis_args.rs not in conflict"
    
    # For binary names in Cargo.toml files, keep ours
    git checkout --ours bin/tempo/Cargo.toml 2>/dev/null || echo "bin/tempo/Cargo.toml not in conflict"
    git checkout --ours bin/tempo-bench/Cargo.toml 2>/dev/null || echo "bin/tempo-bench/Cargo.toml not in conflict"
    git checkout --ours bin/tempo-sidecar/Cargo.toml 2>/dev/null || echo "bin/tempo-sidecar/Cargo.toml not in conflict"
    git checkout --ours xtask/Cargo.toml 2>/dev/null || echo "xtask/Cargo.toml not in conflict"
    
    # For client version name, keep ours
    git checkout --ours crates/node/src/version.rs 2>/dev/null || echo "crates/node/src/version.rs not in conflict"
    
    # For pyroscope app name, keep ours
    git checkout --ours bin/tempo/src/main.rs 2>/dev/null || echo "bin/tempo/src/main.rs not in conflict"
    
    # For workflow files that OtterEVM removed, keep deletion
    git rm .github/workflows/specs.yml 2>/dev/null || echo ".github/workflows/specs.yml not in conflict"
    git rm .github/workflows/test.yml 2>/dev/null || echo ".github/workflows/test.yml not in conflict"
    git rm tempoup/install 2>/dev/null || echo "tempoup/install not in conflict"
    
    # Stage all resolved conflicts
    git add .
    
    # Continue with merge
    if ! git commit -m "merge: resolve conflicts from upstream"; then
        echo "Manual conflict resolution still required. Please resolve remaining conflicts and commit manually."
        echo "Conflicted files:"
        git status --porcelain | grep "^UU\|^UD\|^DU"
        exit 1
    fi
}

echo "[4/5] Pushing otterevm..."
git push origin otterevm

echo "[5/5] Update complete!"
echo ""
echo "Summary of changes:"
echo "- Main branch updated from upstream"
echo "- Otterevm branch merged with main"
echo "- Conflicts resolved automatically where possible"
echo "- Currency identifier kept as 'FEE' (not 'USD')"
echo "- Binary names kept as 'otter*' (not 'tempo*')"
echo "- Branding kept as 'OtterEVM' (not 'Tempo')"
echo ""
echo "Next steps:"
echo "1. Run tests: cargo nextest run"
echo "2. Build binaries: cargo build --bin otter --bin otter-bench --bin otter-sidecar --bin otter-xtask"
echo "3. Verify version: ./target/debug/otter --version"