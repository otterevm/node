[group('deps')]
[doc('Bump all reth dependencies to a specific commit hash')]
bump-reth commit:
    sed -i '' 's/\(reth[a-z_-]* = { git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "\)[a-f0-9]*"/\1{{commit}}"/g' Cargo.toml
    cargo update

[group('deps')]
install-cross:
    cargo install cross --git https://github.com/cross-rs/cross

[group('build')]
[doc('Builds all OtterEVM binaries in cargo release mode')]
build-all-release extra_args="": (build-release "otter" extra_args)

[group('build')]
[doc('Builds all OtterEVM binaries')]
build-all extra_args="": (build "otter" extra_args)

build-release binary extra_args="": (build binary "-r " + extra_args)

build binary extra_args="":
    {{cargo_build_binary}} build {{extra_args}} --bin {{binary}}

[group('localnet')]
[doc('Generates a genesis file')]
genesis accounts="1000" output="./" profile="maxperf":
    cargo run --bin otter-xtask --profile {{profile}} -- generate-genesis --output {{output}} -a {{accounts}} --no-dkg-in-genesis

[group('localnet')]
[doc('Deletes local network data and launches a new localnet')]
[confirm('This will wipe your data directory (unless you have reset=false) - please confirm before proceeding (y/n):')]
localnet accounts="1000" reset="true" profile="maxperf" features="asm-keccak" args="":
    #!/bin/bash
    if [[ "{{reset}}" = "true" ]]; then
        rm -r ./localnet/ || true
        mkdir ./localnet/
        just genesis {{accounts}} ./localnet {{profile}}
    fi;
    cargo run --bin otter --profile {{profile}} --features {{features}} -- \
                      node \
                      --chain ./localnet/genesis.json \
                      --dev \
                      --dev.block-time 1sec \
                      --datadir ./localnet/reth \
                      --http \
                      --http.addr 0.0.0.0 \
                      --http.port 8545 \
                      --http.api all \
                      --engine.disable-precompile-cache \
                      --engine.legacy-state-root \
                      --builder.gaslimit 3000000000 \
                      --builder.max-tasks 8 \
                      --builder.deadline 3 \
                      --log.file.directory ./localnet/logs \
                      --faucet.enabled \
                      --faucet.private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
                      --faucet.amount 1000000000000 \
                      --faucet.address 0x20c0000000000000000000000000000000000001 \
                      {{args}}

mod scripts

[group('dev')]
tempo-dev-up: scripts::tempo-dev-up
tempo-dev-down: scripts::tempo-dev-down
