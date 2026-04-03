# Patching from Tempo

This document describes the modifications made to the upstream [Tempo](https://github.com/tempoxyz/tempo) codebase to create OtterEVM.

## Overview

OtterEVM is forked from Tempo and maintains sync with upstream through the `update-from-tempo.sh` script. This document tracks all customizations and patches applied on top of the Tempo codebase.

## Key Modifications

### 1. Binary Names

All binary names have been renamed from `tempo-*` to `otter-*`:

| Original | OtterEVM |
|----------|----------|
| `tempo` | `otter` |
| `tempo-bench` | `otter-bench` |
| `tempo-sidecar` | `otter-sidecar` |
| `tempo-xtask` | `otter-xtask` |

**Modified Files:**
- `bin/tempo/Cargo.toml` (line 82)
- `bin/tempo-bench/Cargo.toml` (line 11)
- `bin/tempo-sidecar/Cargo.toml` (line 11)
- `xtask/Cargo.toml` (line 14)

**Build Command:**
```bash
cargo build --bin otter --bin otter-bench --bin otter-sidecar --bin otter-xtask
```

### 2. Branding & Version Information

Client identity and versioning have been updated to reflect OtterEVM branding:

- **Client Name:** `"Tempo"` → `"OtterEVM"`
- **CLI Description:** Updated to `"OtterEVM"`
- **Pyroscope App Name:** `"tempo"` → `"otter"` (default value)
- **Version Extra Data:** `"tempo/v{version}/{os}"` → `"otter/v{version}/{os}"`

**Modified Files:**
- `crates/node/src/version.rs`
  - Line 16: `name_client: Cow::Borrowed("OtterEVM")`
  - Line 40: `format!("otter/v{}/{}", ...)`
- `bin/tempo/src/main.rs`
  - Line 109: Pyroscope default application name
  - Line 208: `.about("OtterEVM")`

### 3. Currency Identifier

Fee token currency identifier uses `'FEE'` instead of `'USD'` for OtterEVM's native fee system.

**Modified Files:**
- `crates/precompiles/src/tip20/mod.rs` - Currency validation and handling
- `crates/revm/src/common.rs` - Fee token validation logic (line 207)
- `xtask/src/genesis_args.rs` - Genesis configuration

**Impact:**
- Currency checks in `is_tip20_usd()` validate against `"USD"` for Fee AMM compatibility
- Genesis token creation uses appropriate currency identifiers

### 4. Removed GitHub Workflows

The following CI/CD workflows have been removed as OtterEVM uses different CI infrastructure:

- `.github/workflows/specs.yml` - Specification tests
- `.github/workflows/test.yml` - Test workflows
- `tempoup/install` - Installer script

### 5. Update Script

Added `update-from-tempo.sh` to maintain synchronization with upstream Tempo:

**Purpose:**
- Fetch latest changes from `upstream/main` (tempoxyz/tempo)
- Merge into local `main` branch
- Merge `main` into `otterevm` branch
- Auto-resolve common conflicts (branding, binary names, currency)

**Usage:**
```bash
./update-from-tempo.sh
```

**Conflict Resolution Strategy:**
The script automatically resolves conflicts by keeping OtterEVM customizations:
- Branding files → keep ours (OtterEVM)
- Binary names in Cargo.toml → keep ours (otter-*)
- Currency identifier → keep ours (FEE)
- Version files → keep ours
- Removed workflows → keep deletion

## Syncing with Upstream

### Prerequisites

1. Set up the upstream remote:
```bash
git remote add upstream https://github.com/tempoxyz/tempo.git
```

2. Ensure you're on the `otterevm` branch:
```bash
git checkout otterevm
```

### Update Process

Run the update script:
```bash
./update-from-tempo.sh
```

This will:
1. Fetch from `upstream/main`
2. Merge into `main` (push to origin)
3. Merge `main` into `otterevm` (push to origin)
4. Auto-resolve common conflicts

### Manual Conflict Resolution

If the script encounters conflicts it cannot auto-resolve:

1. Check conflicted files:
```bash
git status --porcelain | grep "^UU\|^UD\|^DU"
```

2. Resolve conflicts manually:
   - For OtterEVM customizations → use `git checkout --ours <file>`
   - For upstream improvements → use `git checkout --theirs <file>`
   - For complex merges → manual editing required

3. Stage resolved files:
```bash
git add <resolved-files>
```

4. Complete the merge:
```bash
git commit -m "merge: resolve conflicts from upstream"
```

### Post-Update Steps

After syncing, always verify:

1. **Run tests:**
```bash
cargo nextest run
```

2. **Build binaries:**
```bash
cargo build --bin otter --bin otter-bench --bin otter-sidecar --bin otter-xtask
```

3. **Verify version:**
```bash
./target/debug/otter --version
```

Expected output should show:
- Client: OtterEVM
- Binary: otter
- Version: appropriate version number

## Architecture Decisions

### Why Fork Tempo?

OtterEVM maintains Tempo's excellent payments-focused architecture while customizing:
- **Branding:** OtterEVM identity across all user-facing outputs
- **Naming:** Consistent `otter-*` binary naming convention
- **Currency Model:** Adapted fee token system for OtterEVM's ecosystem

### What We Preserve from Tempo

- ✅ TIP-20 token standard
- ✅ Fee AMM functionality
- ✅ Tempo Transactions (smart accounts)
- ✅ Commonware consensus integration
- ✅ Reth SDK foundation
- ✅ All protocol-level innovations

### What We Customize

- 🎨 Branding and identity
- 📦 Binary naming
- 💱 Currency identifiers (where applicable)
- 🚫 Removed Tempo-specific CI workflows

## Maintenance Notes

### When Upstream Releases New Version

1. Check upstream release notes for breaking changes
2. Run `./update-from-tempo.sh`
3. Review auto-resolved conflicts
4. Run full test suite
5. Update this document if new patches are needed

### Adding New Patches

If you need to add OtterEVM-specific customizations:

1. **Document it here** - Add to the appropriate section above
2. **Update conflict resolution** - Modify `update-from-tempo.sh` if needed
3. **Test thoroughly** - Ensure patches survive upstream merges
4. **Follow conventions** - Use Conventional Commits for patch commits

### Common Conflict Patterns

The following files frequently conflict during merges:

| File | Conflict Type | Resolution |
|------|--------------|------------|
| `Cargo.toml` files | Binary names | Keep ours (otter-*) |
| `version.rs` | Client name | Keep ours (OtterEVM) |
| `main.rs` | CLI branding | Keep ours |
| `tip20/mod.rs` | Currency checks | Keep ours (FEE) |
| Workflow files | Deletions | Keep deleted |

## Troubleshooting

### Build Fails After Update

```bash
# Clean and rebuild
cargo clean
cargo build --bin otter
```

### Version Shows "Tempo" Instead of "OtterEVM"

Check that these files have correct values:
- `crates/node/src/version.rs` - line 16
- `bin/tempo/src/main.rs` - line 208

### Merge Conflicts Not Auto-Resolved

1. Check if new files need to be added to conflict resolution in `update-from-tempo.sh`
2. Manually resolve using `git checkout --ours/--theirs`
3. Update this document with new conflict patterns

## Related Documentation

- [Tempo Documentation](https://docs.tempo.xyz/)
- [Tempo GitHub](https://github.com/tempoxyz/tempo)
- [AGENTS.md](../AGENTS.md) - Contribution guidelines
- [TIPs](../tips/) - Tempo Improvement Proposals

## History

- **Initial Fork:** Created from Tempo upstream
- **Binary Renaming:** tempo-* → otter-*
- **Branding Update:** Client name and version metadata
- **Currency Adaptation:** FEE identifier for OtterEVM
- **CI Cleanup:** Removed Tempo-specific workflows
- **Update Script:** Automated sync mechanism

---

**Last Updated:** 2026-04-03  
**Maintained By:** OtterEVM Team
