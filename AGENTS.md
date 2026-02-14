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
