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

### Resolution Strategy
```bash
# For branding files, keep ours
git checkout --ours README.md
git add README.md

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
