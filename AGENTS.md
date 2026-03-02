# OtterEVM - Agent Guide

## Project Overview

OtterEVM is a fork of [Tempo](https://github.com/tempoxyz/tempo) - an EVM-compatible blockchain for payments at scale.

### Repository Structure
- **Origin**: `github.com:otterevm/node.git`
- **Upstream**: `https://github.com/tempoxyz/tempo.git`
- **Active Branch**: `otterevm`

---

## Naming Convention

### Binary Names (Changed to OtterEVM)
| Binary | Command |
|--------|---------|
| Main node | `otter` |
| Benchmark tool | `otter-bench` |
| Sidecar | `otter-sidecar` |
| Build tasks | `otter-xtask` |

### Crate Names (Kept as tempo-* for upstream compatibility)
- `tempo-node`, `tempo-evm`, `tempo-consensus`, etc.
- **Reason**: Easier to pull updates from upstream tempo without conflicts

---

## Key Files Modified from Upstream

### Build & Packaging
- `bin/tempo/Cargo.toml` - Binary name: `otter`
- `bin/tempo-bench/Cargo.toml` - Binary name: `otter-bench`
- `bin/tempo-sidecar/Cargo.toml` - Binary name: `otter-sidecar`
- `xtask/Cargo.toml` - Binary name: `otter-xtask`
- `Dockerfile` - Updated binary references
- `docker-bake.hcl` / `docker-bake-profiling.hcl` - Target names
- `Justfile` - Build commands

### Documentation & Branding
- `README.md` - OtterEVM branding
- `crates/node/src/version.rs` - `name_client: "OtterEVM"`
- `bin/tempo/src/main.rs` - Pyroscope default app name
- Various `crates/*/src/lib.rs` - Doc comments ("Tempo" → "OtterEVM")

### Scripts & Tools
- `scripts/Justfile` - Commands updated
- `scripts/*.sh` - `tempo_fundAddress` → `otter_fundAddress`
- `scripts/consensus/*.sh` - Container names: `otter-validator-*`
- `otter.nu` (was `tempo.nu`) - Nushell utilities
- `otterup/` (was `tempoup/`) - Installer scripts

---

## Build Commands

```bash
# Build all binaries
cargo build --bin otter --bin otter-bench --bin otter-sidecar --bin otter-xtask

# Build release
cargo build --release --bin otter

# Using just
just build-all
just build-all-release

# Using nushell
nu otter.nu localnet  # Start localnet
nu otter.nu bench     # Run benchmarks
```

### Build Outputs
```
target/debug/otter
target/debug/otter-bench
target/debug/otter-sidecar
target/debug/otter-xtask
```

---

## Update from Upstream (Tempo)

### Setup (one-time)
```bash
git remote add upstream https://github.com/tempoxyz/tempo.git
```

### Update Workflow
```bash
# 1. Fetch upstream
git fetch upstream

# 2. Update main branch
git checkout main
git merge upstream/main
git push origin main

# 3. Update otterevm branch
git checkout otterevm
git merge main

# 4. Resolve conflicts if any
# - README.md: Keep OtterEVM branding
# - bin/tempo*/Cargo.toml: Keep binary names (otter*)
# - crates/node/src/version.rs: Keep "OtterEVM"
# - Other files: Merge normally

# 5. Push
git push origin otterevm
```

### Auto-update Script
Save as `update-from-tempo.sh`:
```bash
#!/bin/bash
set -e

echo "[1/4] Fetching upstream..."
git fetch upstream

echo "[2/4] Updating main..."
git checkout main
git merge upstream/main --no-edit || {
    echo "Merge conflicts in main. Resolve manually."
    exit 1
}
git push origin main

echo "[3/4] Updating otterevm..."
git checkout otterevm
git merge main --no-edit || {
    echo "Merge conflicts in otterevm. Resolve manually."
    echo "Common conflicts: README.md, Cargo.toml files, version.rs"
    exit 1
}
git add .

echo "[4/4] Pushing otterevm..."
git push origin otterevm

echo "✓ Update complete!"
```

---

## Testing

```bash
# Run tests
cargo nextest run

# Run specific test
cargo test -p tempo-node

# Start localnet
just localnet
# or
nu otter.nu localnet

# Check version
./target/debug/otter --version
# Expected: OtterEVM Version: x.x.x
```

---

## Common Issues

### Merge Conflicts Expected In
1. `README.md` - Branding differences
2. `bin/tempo/Cargo.toml` - Binary name
3. `crates/node/src/version.rs` - Client name
4. `Justfile` - Command names
5. **Currency identifier files** - USD vs FEE (OtterEVM uses "FEE", Tempo uses "USD"):
   - `crates/precompiles/src/tip20/mod.rs` - `USD_CURRENCY` constant
   - `crates/revm/src/common.rs` - Fee token validation
   - `xtask/src/genesis_args.rs` - pathUSD/pathFEE token naming

### Resolution Strategy
```bash
# For branding files, keep ours
git checkout --ours README.md
git add README.md

# For currency identifier (we use FEE, upstream uses USD), keep ours
git checkout --ours crates/precompiles/src/tip20/mod.rs
git checkout --ours crates/revm/src/common.rs
git checkout --ours xtask/src/genesis_args.rs
git add crates/precompiles/src/tip20/mod.rs crates/revm/src/common.rs xtask/src/genesis_args.rs

# For upstream code changes, keep theirs
git checkout --theirs crates/some-crate/src/lib.rs
git add crates/some-crate/src/lib.rs

# Then commit
git commit -m "merge: resolve conflicts from upstream"
```

---

## Docker

### Build Configuration
- **Dockerfile**: Multi-stage build with 6 targets
- **Base Image**: `debian:bookworm-slim`
- **Chef Image**: Pre-compiles dependencies for caching
- **Build Time**: 10-30 minutes (first time)

### Available Targets
| Target | Description |
|--------|-------------|
| `otter` | Main node binary |
| `otter-bench` | Benchmark tool (includes nushell) |
| `otter-sidecar` | Sidecar service |
| `otter-xtask` | Build/utility tasks |

### Build Commands

```bash
# Build all targets
docker buildx bake

# Build specific target
docker buildx bake otter
docker buildx bake otter-bench

# Build and load to local docker
docker buildx bake --load otter

# Build with custom profile
docker buildx bake --set *.args.RUST_PROFILE=release

# Using Dockerfile directly
docker build --target otter -t otter:latest .
docker build --target otter-bench -t otter-bench:latest .
```

### Run Containers

```bash
# Run main node
docker run -it --rm otter:latest --help
docker run -it --rm -p 8545:8545 otter:latest node --dev --http

# Run with volume for data
docker run -it --rm -v $(pwd)/data:/data otter:latest node --datadir /data

# Run sidecar
docker run -it --rm otter-sidecar:latest --help

# Run benchmark
docker run -it --rm otter-bench:latest --help
```

### Build Stages
```dockerfile
1. chef      - Cache dependencies
2. builder   - Compile all binaries
3. base      - Minimal Debian image
4. otter     - Copy otter binary
5. otter-*   - Copy other binaries
```

### Known Warnings (Non-critical)
- `DL3045`: COPY without WORKDIR (accepted pattern)
- `DL3008`: apt-get without version pins (acceptable)

### Troubleshooting

```bash
# Clean build cache if issues
docker buildx prune -f

# Check build configuration
docker buildx bake --print

# Build with verbose output
docker buildx bake --progress=plain otter
```

---

## Adding a New Chain

This section documents how to add a new chain to OtterEVM while keeping existing chains.

### Prerequisites

- Genesis JSON file for the new chain
- Unique Chain ID (check existing: 4217=presto, 42429=andantino, 42431=moderato)
- Bootnode enode URLs (at least 1, recommended 3-4)
- (Optional) Snapshot download URL
- (Optional) RPC endpoint URL

### Step-by-Step Guide

#### Step 1: Create Genesis File

```bash
# Create the genesis JSON file
touch crates/chainspec/src/genesis/{chain_name}.json
```

**Required fields in genesis.json:**
```json
{
  "config": {
    "chainId": {unique_chain_id},
    "t0Time": 0,
    "t1Time": 0,
    "t2Time": null,
    "epochLength": 100
  },
  "nonce": "0x42",
  "timestamp": "0x0",
  "extraData": "0x",
  "gasLimit": "0xb2d05e00",
  "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
  "alloc": {
    // Pre-deployed contracts and funded accounts
  }
}
```

**Important:** Do NOT include `hash` field - it will be calculated automatically from the content.

#### Step 2: Add Bootnodes

Edit `crates/chainspec/src/bootnodes.rs`:

```rust
// Add after existing bootnode definitions
pub(crate) static {CHAIN_NAME}_BOOTNODES: [&str; N] = [
    "enode://pubkey1@ip1:port1",
    "enode://pubkey2@ip2:port2",
    // Add more nodes as needed
];

pub(crate) fn {chain_name}_nodes() -> Vec<NodeRecord> {
    parse_nodes({CHAIN_NAME}_BOOTNODES)
}
```

#### Step 3: Register Chain in spec.rs

Edit `crates/chainspec/src/spec.rs`:

**3.1 Add to import:**
```rust
use crate::bootnodes::{
    andantino_nodes,
    moderato_nodes,
    presto_nodes,
    {chain_name}_nodes,  // Add this
};
```

**3.2 Add to SUPPORTED_CHAINS:**
```rust
pub const SUPPORTED_CHAINS: &[&str] = &[
    "mainnet",
    "moderato",
    "testnet",
    "{chain_name}",  // Add this
];
```

**3.3 Add to chain_value_parser:**
```rust
pub fn chain_value_parser(s: &str) -> eyre::Result<Arc<TempoChainSpec>> {
    Ok(match s {
        "mainnet" => PRESTO.clone(),
        "testnet" => ANDANTINO.clone(),
        "moderato" => MODERATO.clone(),
        "dev" => DEV.clone(),
        "{chain_name}" => {CHAIN_NAME}.clone(),  // Add this
        _ => TempoChainSpec::from_genesis(...)?,
    })
}
```

**3.4 Create static variable:**
```rust
pub static {CHAIN_NAME}: LazyLock<Arc<TempoChainSpec>> = LazyLock::new(|| {
    let genesis: Genesis = serde_json::from_str(
        include_str!("./genesis/{chain_name}.json")
    ).expect("`./genesis/{chain_name}.json` must be present");

    TempoChainSpec::from_genesis(genesis)
        .with_default_follow_url("wss://rpc.{chain_name}.otterevm.xyz")
        .into()
});
```

**3.5 Add to bootnodes() match:**
```rust
fn bootnodes(&self) -> Option<Vec<NodeRecord>> {
    match self.inner.chain_id() {
        4217 => Some(presto_nodes()),
        42429 => Some(andantino_nodes()),
        42431 => Some(moderato_nodes()),
        {chain_id} => Some({chain_name}_nodes()),  // Add this
        _ => self.inner.bootnodes(),
    }
}
```

#### Step 4: Add Snapshot URL (Optional)

Edit `bin/tempo/src/defaults.rs`:

```rust
let download_defaults = DownloadDefaults {
    available_snapshots: vec![
        Cow::Borrowed("https://snapshots.tempoxyz.dev/42431 (moderato)"),
        Cow::Borrowed("https://snapshots.tempoxyz.dev/42429 (andantino)"),
        Cow::Borrowed("https://snapshots.otterevm.xyz/{chain_id} ({chain_name})"),  // Add
    ],
    default_base_url: Cow::Borrowed(DEFAULT_DOWNLOAD_URL),
    long_help: None,
};
```

#### Step 5: Update CLI Help (Optional)

Edit `bin/tempo/src/tempo_cmd.rs`:

```rust
/// Chain to query (presto, testnet, moderato, {chain_name}, or path to chainspec)
#[arg(long, short, default_value = "mainnet", value_parser = ...)]
chain: Arc<TempoChainSpec>,

/// RPC URL to query
#[arg(long, default_value = "https://rpc.{chain_name}.otterevm.xyz")]
rpc_url: String,
```

#### Step 6: Add Test (Optional)

Edit `crates/chainspec/src/spec.rs` in tests module:

```rust
#[test]
fn can_load_{chain_name}() {
    let _ = super::TempoChainSpecParser::parse("{chain_name}")
        .expect("the {chain_name} chainspec must always be well formed");
}
```

### Testing the New Chain

```bash
# Build
cargo build --bin otter

# Test chain parsing
./target/debug/otter node --chain {chain_name} --help

# Run node with new chain
./target/debug/otter node --chain {chain_name} --dev --http

# Test validators info (if RPC is available)
./target/debug/otter consensus validators-info --chain {chain_name}

# Run tests
cargo test -p tempo-chainspec can_load_{chain_name}
```

### Important Notes

1. **Chain ID must be unique** - Check against existing: 4217, 42429, 42431
2. **Genesis hash is calculated automatically** - Do not hardcode in source
3. **Changing genesis.json content changes the hash** - This creates a different chain
4. **Bootnodes must be accessible** - Otherwise node can't find peers
5. **Keep backup of genesis.json** - Lost = can't recreate same chain

### Example: Complete Chain Addition

See existing chains as examples:
- `crates/chainspec/src/genesis/moderato.json`
- `crates/chainspec/src/genesis/andantino.json`
- `crates/chainspec/src/genesis/presto.json`

---

## Pull Requests

### Titles

Use [Conventional Commits](https://www.conventionalcommits.org/) with an optional scope:

```
<type>(<scope>): <short description>
```

**Types**: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore`

**Scope** (optional): crate or area, e.g. `evm`, `consensus`, `rpc`, `tip-1017`

Examples:
- `fix(rpc): correct gas estimation for TIP-20 transfers`
- `perf: batch trie updates to reduce cursor overhead`
- `feat(consensus): add checkpoint guard for batched state ops`

### Descriptions

Keep it short. Say what changed and why — nothing more.

**Do:**
- Write 1–3 sentences summarizing the change
- Explain _why_ if the diff doesn't make it obvious
- Link related issues or TIPs
- Include benchmark numbers for perf changes

**Don't:**
- List every file changed — that's what the diff is for
- Repeat the title in the body
- Add "Files changed" or "Changes" sections
- Write walls of text that go stale when the diff is updated
- Use filler like "This PR introduces...", "comprehensive", "robust", "enhance", "leverage"

**Template:**

```
Closes #<issue>

<what changed, 1-3 sentences>

<why, if not obvious from the diff>
```

**Good example:**

```
Closes #2901

Adds `valid_before` upper bound for all AA transactions. Transactions past
their expiry are rejected at validation time and cleaned up from the pool
via a periodic sweep.
```

**Bad example:**

```
## Summary
This PR introduces comprehensive validation checks for the valid_before field.

## Changes
- Modified `crates/pool/src/validate.rs` to add validation
- Modified `crates/pool/src/pool.rs` to add cleanup
- Added tests in `crates/pool/src/tests/valid_before.rs`

## Files Changed
- crates/pool/src/validate.rs
- crates/pool/src/pool.rs
- crates/pool/src/tests/valid_before.rs
```

---

## Quick Reference

| Task | Command |
|------|---------|
| Build all | `just build-all` |
| Build release | `just build-all-release` |
| Test | `cargo nextest run` |
| Localnet | `just localnet` or `nu otter.nu localnet` |
| Update from tempo | `./update-from-tempo.sh` |
| Check version | `./target/debug/otter --version` |

---

## Node Upgrade Guide

This section documents how to upgrade OtterEVM nodes between versions, particularly from v1.2.x to v1.3.x.

### Critical: Hardfork Compatibility

The most important factor for successful upgrades is **genesis hardfork configuration**.

#### ✅ Compatible Upgrade (No Issues)

```json
{
  "config": {
    "chainId": 7441,
    "t0Time": 0,
    "t1Time": 0
  }
}
```

When genesis only has `t0Time` and `t1Time`, upgrades work seamlessly because:
- No T2 hardfork activation
- No ValidatorConfigV2 bytecode deployment
- State root calculation remains consistent across versions

#### ❌ Incompatible Upgrade (Will Fail)

```json
{
  "config": {
    "chainId": 7441,
    "t0Time": 0,
    "t1Time": 0,
    "t2Time": 0
  }
}
```

When `t2Time` is set to 0 (or any past timestamp), the T2 hardfork activates immediately:
- v1.3.x deploys 0xEF marker bytecode to `VALIDATOR_CONFIG_V2_ADDRESS` in `apply_pre_execution_changes()`
- v1.2.x does NOT deploy this bytecode
- **Result**: State root mismatch between versions

### Pre-Upgrade Checklist

1. **Verify genesis.json hardfork times:**
   ```bash
   grep -E '"t[0-9]Time"' genesis.json
   ```
   - Ensure no `t2Time` or set it to future timestamp

2. **Backup data directory:**
   ```bash
   cp -r data/ data.backup.$(date +%Y%m%d)/
   ```

3. **Check current block height:**
   ```bash
   ./otter --version
   tail -f logs | grep "Block added"
   ```

4. **Prepare new binary:**
   ```bash
   # Build v1.3 binary
   git checkout v1.3.1  # or otterevm branch
   cargo build --release --bin otter
   cp target/release/otter ./otter-1.3
   ```

### Upgrade Procedure

#### Step 1: Stop Current Node
```bash
# Graceful shutdown
pkill -SIGTERM -f "otter"

# Verify stopped
ps aux | grep otter | grep -v grep
```

#### Step 2: Verify Data Integrity
```bash
# Check last finalized block
ls -la data/consensus/
ls -la data/db/
```

#### Step 3: Switch Binary
```bash
# Option A: Replace binary
mv otter otter-1.2.backup
mv otter-1.3 otter

# Option B: Use explicit version
./otter-1.3 node --datadir data/ ...
```

#### Step 4: Start New Version
```bash
./otter-1.3 node \
  --consensus.signing-key keys/signing.key \
  --consensus.signing-share keys/signing.share \
  --chain genesis.json \
  --datadir data/ \
  --consensus.fee-recipient 0x... \
  --http --http.addr 0.0.0.0 --http.port 8545 \
  --port 30303 \
  --discovery.port 30304
```

#### Step 5: Verify Success
Watch for these log messages:
```
✅ Block added to canonical chain number={N+1}
✅ Constructed proposal proposal.height={N+2}
✅ No "state root mismatch" errors
```

### Troubleshooting Upgrades

#### Issue: "proposal return channel was closed"

**Symptoms:**
```
WARN handle_propose: error=[proposal return channel was closed by consensus engine...]
```

**Causes & Solutions:**

| Cause | Solution |
|-------|----------|
| Consensus layer catching up | Wait 30-60 seconds, usually resolves automatically |
| Port conflicts | Use different `--port` and `--discovery.port` |
| State root mismatch | **Fatal** - Check if t2Time is set in genesis |

#### Issue: State Root Mismatch (Fatal)

**Symptoms:**
```
ERROR: State root mismatch
Expected: 0x5a5745ed6bed0f7e8500247c9014fb49a9644c5f4b2813c307875beb456156e8
Actual:   0xbc3085672aa756336bcafe28622085b75520367df28e7e74a389e15666e54788
```

**Root Cause:** T2 hardfork active (`t2Time: 0` in genesis)

**Solutions:**
1. **Best**: Modify genesis to remove/disable t2Time (requires chain reset)
2. **Alternative**: Stay on v1.2.x indefinitely
3. **Not Recommended**: Patch v1.3 to skip T2 validation (security risk)

#### Issue: Port Already in Use

**Solution:** Use different ports for testing:
```bash
./otter-1.3 node \
  --port 30305 \
  --discovery.port 30306 \
  --http.port 8546
```

### Testing Upgrades

Before production deployment, test in this order:

1. **Local Test** (Fresh chain):
   ```bash
   rm -rf test-data/
   ./otter-1.2 node --datadir test-data/ ...  # Run 5 mins
   pkill otter-1.2
   ./otter-1.3 node --datadir test-data/ ...  # Verify continues
   ```

2. **Snapshot Test** (Existing chain):
   ```bash
   # Use copy of production data
   cp -r prod-data/ test-data/
   ./otter-1.3 node --datadir test-data/ ...
   ```

3. **Staging Test** (Full replica):
   - Deploy to staging environment
   - Run for 24+ hours
   - Monitor block production

### Rollback Procedure

If upgrade fails:

```bash
# 1. Stop new version
pkill -f "otter-1.3"

# 2. Restore from backup
rm -rf data/
cp -r data.backup.20250302/ data/

# 3. Start old version
./otter-1.2 node --datadir data/ ...
```

### Version-Specific Notes

#### v1.2.0 → v1.3.0
- **Reth Upgrade**: v1.10.x → v1.11.x
- **Breaking API**: `ReceiptRootBloom` parameter added to consensus validation
- **New Behavior**: `PayloadStatusEnum::Accepted` now rejected (only `Valid` accepted)
- **Compatible if**: No t2Time in genesis

#### v1.3.x Future Upgrades
- Always check `crates/evm/src/block.rs` for `apply_pre_execution_changes()` modifications
- Hardfork times in genesis are the primary compatibility factor

### Quick Reference: Upgrade Commands

```bash
# Pre-upgrade backup
tar -czf backup-$(date +%Y%m%d).tar.gz data/ keys/ genesis.json

# Version check
./otter --version

# Start with logs
tee -a otter.log | ./otter node ...

# Monitor blocks
tail -f otter.log | grep "Block added"
```

---

## Notes for AI Agents

1. **Always check this file first** when starting a new session
2. **Binary names are `otter*` not `tempo*`** when building/running
3. **Crate names remain `tempo-*`** for Cargo.toml dependencies
4. **Main branch** tracks upstream; **otterevm branch** has our changes
5. When updating from upstream, expect conflicts in branding files
6. **Currency identifier is "FEE" not "USD"** - Always keep OtterEVM's FEE changes:
   - `USD_CURRENCY = "FEE"` (not "USD")
   - Token name: `pathFEE` (not `pathUSD`)
   - Fee token validation checks for "FEE"