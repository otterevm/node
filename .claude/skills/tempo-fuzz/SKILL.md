---
name: tempo-fuzz
description: Running fuzz tests for Tempo precompiles using tempo-foundry. Use when testing Rust precompile implementations or syncing with Solidity implementations.
---

# Tempo Fuzz Testing

## Overview

Tempo uses a dual-testing approach to ensure Rust precompile implementations match Solidity reference implementations. The same test suite runs against both:

1. **Solidity implementations** - Using standard Foundry (`forge`)
2. **Rust precompile implementations** - Using tempo-foundry's custom `forge` binary

## Repository Structure

```
Tempo/
├── tempo/                    # Main Tempo node (Rust) - often has multiple git worktrees
│   └── docs/specs/          # Solidity specs and fuzz tests
│       ├── src/             # Solidity reference implementations
│       ├── test/            # Test files (*.t.sol)
│       ├── tempo-forge      # Script to run tempo-foundry's forge
│       ├── tempo-cast       # Script to run tempo-foundry's cast
│       └── foundry.toml     # Foundry configuration
└── tempo-foundry/           # Foundry fork with Tempo's EVM (sibling directory)
```

## How the `isTempo` Flag Works

The test suite uses an `isTempo` flag (defined in `BaseTest.t.sol`) to detect which implementation is being tested:

```solidity
isTempo = _TIP403REGISTRY.code.length + _TIP20FACTORY.code.length + _PATH_USD.code.length
        + _STABLECOIN_DEX.code.length + _NONCE.code.length > 0;
```

- **`isTempo = false`**: No precompile code exists at expected addresses. Tests deploy Solidity implementations via `deployCodeTo()`. This is the default when using standard `forge`.
- **`isTempo = true`**: Native precompile code exists at addresses like `0x403c...` (TIP403Registry), `0x20Fc...` (TIP20Factory). Tests run against Rust precompiles built into tempo-foundry's EVM.

### Precompile Addresses

| Precompile | Address | Notes |
|------------|---------|-------|
| TIP403Registry | `0x403c000000000000000000000000000000000000` | |
| TIP20Factory | `0x20Fc000000000000000000000000000000000000` | |
| PathUSD | `0x20C0000000000000000000000000000000000000` | TIP20 at token_id=0 |
| StablecoinExchange | `0xDEc0000000000000000000000000000000000000` | |
| FeeManager | `0xfeEC000000000000000000000000000000000000` | Also called FeeAMM |
| Nonce | `0x4e4F4E4345000000000000000000000000000000` | |
| ValidatorConfig | `0xCccCcCCC00000000000000000000000000000000` | |
| AccountKeychain | `0xAAAAAAAA00000000000000000000000000000000` | Access key management |

## Running Tests

### Option 1: Test Against Solidity Implementations (Standard Foundry)

```bash
cd docs/specs

# Run all tests
forge test

# Verbose output
forge test -vvv

# Run specific test
forge test --match-test test_mint

# Run tests in a specific file
forge test --match-path test/TIP20.t.sol
```

### Option 2: Test Against Rust Precompiles (tempo-foundry)

```bash
cd docs/specs

# Run all tests against Rust precompiles
./tempo-forge test

# Verbose output
./tempo-forge test -vvv

# Run specific test
./tempo-forge test --match-test test_mint

# Run tests in a specific file
./tempo-forge test --match-path test/TIP20.t.sol

# Build contracts only
./tempo-forge build
```

## Setting Up tempo-foundry

### Option 1: Clone as Sibling Directory (Recommended)

```bash
# From tempo repo root
cd ..
git clone git@github.com:tempoxyz/tempo-foundry.git
```

### Option 2: Set Environment Variable

```bash
export TEMPO_FOUNDRY_PATH=/path/to/tempo-foundry
./tempo-forge test
```

### Building tempo-foundry Manually

The `tempo-forge` script auto-builds on first run, but you can build manually:

```bash
cd /path/to/tempo-foundry
cargo build -p forge --profile dev
cargo build -p cast --profile dev
```

If you encounter build errors:

```bash
cargo clean
cargo build -p forge --profile dev
```

## Using tempo-cast

The `tempo-cast` script runs cast commands using tempo-foundry's custom cast binary:

```bash
# Get function signature
./tempo-cast sig "transfer(address,uint256)"

# Decode function selector
./tempo-cast 4byte 0xa9059cbb

# ABI encode
./tempo-cast abi-encode "transfer(address,uint256)" 0x1234...5678 1000000
```

## Writing Tests

### Test File Location

All test files go in `docs/specs/test/` with the `.t.sol` extension.

### Base Test Contract

All tests should inherit from `BaseTest`:

```solidity
import { BaseTest } from "./BaseTest.t.sol";

contract MyTest is BaseTest {
    function setUp() public override {
        super.setUp();
        // Your setup code
    }
}
```

### Handling Implementation Differences

Use `isTempo` to handle differences between Solidity and Rust implementations:

```solidity
function testTransfer() public {
    // Some event expectations only work in Solidity mode
    if (!isTempo) {
        vm.expectEmit(true, true, true, true);
        emit Transfer(alice, bob, amount);
    }
    
    token.transfer(bob, amount);
    
    // Assertions work in both modes
    assertEq(token.balanceOf(bob), amount);
}
```

### Fuzz Tests

Fuzz tests use the `testFuzz_` prefix:

```solidity
function testFuzz_setRewardRecipient(address recipient) public {
    vm.prank(alice);
    token.setRewardRecipient(recipient);
    assertEq(token.rewardRecipient(alice), recipient);
}
```

### Available Test Helpers

From `BaseTest.t.sol`:

- `admin`, `alice`, `bob`, `charlie` - Common test addresses
- `factory` - TIP20Factory instance
- `pathUSD` - PathUSD token instance
- `exchange` - StablecoinExchange instance
- `registry` - TIP403Registry instance
- `nonce` - Nonce precompile instance
- `validatorConfig` - ValidatorConfig instance
- `amm` - FeeManager instance (at `_FEE_AMM` address)
- `token1`, `token2` - Pre-created test tokens
- Role constants: `_ISSUER_ROLE`, `_PAUSE_ROLE`, `_UNPAUSE_ROLE`, `_TRANSFER_ROLE`, `_RECEIVE_WITH_MEMO_ROLE`

### Test Files

Current test files in `docs/specs/test/`:
- `AccountKeychain.t.sol` - Interface tests using mocks
- `BaseTest.t.sol` - Base test framework
- `FeeAMM.t.sol`, `FeeManager.t.sol` - Fee/AMM precompile tests
- `Nonce.t.sol` - Nonce precompile tests
- `StablecoinExchange.t.sol` - DEX precompile tests
- `TIP20.t.sol`, `TIP20Factory.t.sol`, `TIP20RolesAuth.t.sol` - Token tests
- `TIP403Registry.t.sol` - Registry tests
- `ValidatorConfig.t.sol` - Validator config tests

## CI Integration

CI runs both test modes to ensure implementations stay in sync (see `.github/workflows/docs-specs.yml`):

1. **Forge Build** - Builds Solidity contracts
2. **Forge Fmt** - Checks Solidity formatting
3. **Forge Test (Solidity)** - Validates Solidity implementations with `forge test -vvv`
4. **Forge Test (Rust Precompiles)** - Validates Rust precompiles match Solidity specs

### How Rust Precompile CI Works

The CI automatically:
1. Checks out both `tempo` and `tempo-foundry` repos
2. Updates tempo-foundry's `Cargo.toml` to point to the PR's commit SHA
3. Builds tempo-foundry's forge binary
4. Runs tests with `isTempo=true`

**Note:** The tempo-foundry-test job is in `allowed-failures` since it depends on external repo state. Failures don't block merge but should be investigated.

## Troubleshooting

### tempo-foundry not found

```
Error: Could not find tempo-foundry repository.
```

Either clone tempo-foundry as a sibling directory or set `TEMPO_FOUNDRY_PATH`.

### Missing precompile errors

```
MissingPrecompile("TIP403Registry", 0x403c...)
```

This means you're running with tempo-foundry but a precompile is missing. Ensure tempo-foundry is up to date with the latest precompile implementations.

### Build failures

```bash
cd /path/to/tempo-foundry
cargo clean
cargo build -p forge --profile dev
```

### Test passes in Solidity but fails in Rust

This indicates an implementation mismatch - the exact purpose of this testing setup. Debug by:

1. Running with `-vvv` for verbose output
2. Checking if `isTempo` conditional logic is correct
3. Comparing return values and state changes between modes
