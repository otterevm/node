variable "VERGEN_GIT_SHA" {
  default = ""
}

variable "VERGEN_GIT_SHA_SHORT" {
  default = ""
}

group "default" {
  targets = ["otterevm", "tempo-bench", "tempo-sidecar", "tempo-xtask"]
}

target "docker-metadata" {}

# Base image with all dependencies pre-compiled
target "chef" {
  dockerfile = "Dockerfile.chef"
  context = "."
  platforms = ["linux/amd64", "linux/arm64"]
  args = {
    RUST_PROFILE = "profiling"
    RUST_FEATURES = "asm-keccak,jemalloc,otlp"
  }
}

target "_common" {
  dockerfile = "Dockerfile"
  context = "."
  contexts = {
    chef = "target:chef"
  }
  args = {
    CHEF_IMAGE = "chef"
    RUST_PROFILE = "profiling"
    RUST_FEATURES = "asm-keccak,jemalloc,otlp"
    VERGEN_GIT_SHA = "${VERGEN_GIT_SHA}"
    VERGEN_GIT_SHA_SHORT = "${VERGEN_GIT_SHA_SHORT}"
  }
  platforms = ["linux/amd64", "linux/arm64"]
}

target "otterevm" {
  inherits = ["_common", "docker-metadata"]
  target = "otterevm"
}

target "tempo-bench" {
  inherits = ["_common", "docker-metadata"]
  target = "tempo-bench"
}

target "tempo-sidecar" {
  inherits = ["_common", "docker-metadata"]
  target = "tempo-sidecar"
}

target "tempo-xtask" {
  inherits = ["_common", "docker-metadata"]
  target = "tempo-xtask"
}
