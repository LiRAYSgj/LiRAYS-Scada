#!/usr/bin/env bash
set -euo pipefail

# Build all deliverables from macOS:
# - macOS package (runs locally with make)
# - Debian amd64 and arm64 .deb packages (via Docker + QEMU)
#
# Requirements:
# - macOS with Xcode command line tools (toolchains/SDK)
# - Docker Desktop (with binfmt/QEMU for multi-platform)
# - Rust toolchain, Node 24 (nvm or system), npm, protobuf on host
#
# Outputs land in ./distributions/

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$ROOT/distributions"
# Scrape version from Cargo.toml unless provided via env
VERSION="${VERSION:-$(grep -m1 '^version' "$ROOT/Cargo.toml" | awk -F '\"' '{print $2}')}"

run_docker_build() {
  local platform="$1"
  echo "==> Building .deb for $platform"
  docker run --rm --platform="$platform" \
    -v "$ROOT":/src -w /src \
    rust:1.94-bullseye \
    bash -lc "set -e; \
      export DEBIAN_FRONTEND=noninteractive; \
      apt-get update; \
      apt-get install -y curl ca-certificates gnupg; \
      curl -fsSL https://deb.nodesource.com/setup_24.x | bash -; \
      apt-get install -y nodejs protobuf-compiler debhelper devscripts equivs pkg-config libssl-dev build-essential; \
      mk-build-deps -i -r -t 'apt-get -y --no-install-recommends' ./debian/control; \
      export PATH=\"/usr/local/cargo/bin:\$PATH\"; \
      make clean; \
      VERSION=$VERSION make"
}

main() {
  mkdir -p "$DIST_DIR"

  # macOS package (clean build)
  echo "==> Building macOS (host) with make"
  (cd "$ROOT" && VERSION="$VERSION" make clean && VERSION="$VERSION" make)

  # Debian packages via Docker (amd64 & arm64)
  run_docker_build linux/amd64
  run_docker_build linux/arm64

  echo "==> Done. Artifacts in $DIST_DIR"
  ls -lh "$DIST_DIR"
}

main
