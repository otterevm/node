# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OtterEVM is an EVM-compatible blockchain designed specifically for stablecoin payments, forked from Tempo. It's built on the Reth SDK and uses Simplex Consensus via Commonware for fast finality. The architecture focuses on high throughput, low cost, and features that financial institutions and payment service providers expect.

## Architecture

- **Execution Layer**: Based on Reth (Ethereum execution client) for transaction processing and state management
- **Consensus Layer**: Commonware consensus engine for block agreement (Simplex Consensus)
- **Precompiles**: Custom precompiled contracts for payment-specific functionality (TIP-20, TIP-403, etc.)
- **Binary Name**: `otter` (not `tempo`) - this is a branded fork from Tempo
- **Key Features**: TIP-20 token standard, Fee AMM, Tempo Transactions (batched payments), keychain management

## Development Commands

### Building
- `just build-all` - Builds all OtterEVM binaries in debug mode
- `just build-all-release` - Builds all OtterEVM binaries in release mode
- `cargo build --bin otter` - Build the main binary directly

### Testing
- `cargo nextest run` - Run all tests
- `cargo test <test_name>` - Run a specific test
- `cargo nextest run --package <crate_name>` - Run tests for a specific crate

### Local Development
- `just localnet` - Starts a local development network with 1000 accounts
- `just genesis [num_accounts] [output_dir]` - Generates a genesis file
- `cargo run --bin otter node -- --dev` - Run a development node

### Development Scripts
- `just tempo-dev-up` - Start development environment
- `just tempo-dev-down` - Stop development environment

## Key Crates

- `bin/tempo`: Main executable (builds to `otter` binary)
- `crates/node`: Core node implementation bridging Reth and Commonware
- `crates/evm`: EVM configuration and execution logic
- `crates/consensus`: Tempo-specific consensus implementation
- `crates/precompiles`: Payment-focused precompiled contracts (TIP-20, Fee AMM, etc.)
- `crates/primitives`: Core data structures and types
- `crates/transaction-pool`: Custom transaction pool with Tempo-specific logic
- `crates/chainspec`: Chain specification and configuration

## Important Notes

- This is branded as OtterEVM but forked from Tempo codebase - binary is `otter`, currency identifiers may be "FEE" instead of "USD"
- The main executable is named `otter` despite the package name being `tempo` (see `bin/tempo/Cargo.toml`)
- Uses a hybrid architecture: Reth for execution, Commonware for consensus
- Includes payment-specific features like TIP-20 tokens, Fee AMM, and Tempo Transactions
- Requires Rust 1.93.0+
- Dependencies are managed via Cargo workspace with git patches