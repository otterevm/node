[group('deps')]
[doc('Bump all reth dependencies to a specific commit hash')]
bump-reth commit:
    sed -i '' 's/\(reth[a-z_-]* = { git = "https:\/\/github.com\/paradigmxyz\/reth", rev = "\)[a-f0-9]*"/\1{{commit}}"/g' Cargo.toml
    cargo update

mod scripts

[group('dev')]
otter-dev-up: scripts::otter-dev-up
otter-dev-down: scripts::otter-dev-down
