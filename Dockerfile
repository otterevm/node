ARG CHEF_IMAGE=chef

FROM ${CHEF_IMAGE} AS builder

ARG RUST_PROFILE=profiling
ARG VERGEN_GIT_SHA
ARG VERGEN_GIT_SHA_SHORT
ARG EXTRA_RUSTFLAGS=""

COPY . .

# Build ALL binaries in one pass - they share compiled artifacts
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked,id=cargo-registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked,id=cargo-git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked,id=sccache \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold ${EXTRA_RUSTFLAGS}" \
    cargo build --profile ${RUST_PROFILE} \
        --bin otter --features "asm-keccak,jemalloc,otlp" \
        --bin otter-bench \
        --bin otter-sidecar \
        --bin otter-xtask

FROM debian:bookworm-slim AS base

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /data

# otter
FROM base AS otter
ARG RUST_PROFILE=profiling
COPY --from=builder /app/target/${RUST_PROFILE}/otter /usr/local/bin/otter
ENTRYPOINT ["/usr/local/bin/otter"]

# otter-sidecar
FROM base AS otter-sidecar
ARG RUST_PROFILE=profiling
COPY --from=builder /app/target/${RUST_PROFILE}/otter-sidecar /usr/local/bin/otter-sidecar
ENTRYPOINT ["/usr/local/bin/otter-sidecar"]

# otter-xtask
FROM base AS otter-xtask
ARG RUST_PROFILE=profiling
COPY --from=builder /app/target/${RUST_PROFILE}/otter-xtask /usr/local/bin/otter-xtask
ENTRYPOINT ["/usr/local/bin/otter-xtask"]

# otter-bench (needs nushell)
FROM base AS otter-bench
ARG RUST_PROFILE=profiling
COPY --from=ghcr.io/nushell/nushell:0.108.0-bookworm /usr/bin/nu /usr/bin/nu
COPY --from=builder /app/target/${RUST_PROFILE}/otter-bench /usr/local/bin/otter-bench
ENTRYPOINT ["/usr/local/bin/otter-bench"]
