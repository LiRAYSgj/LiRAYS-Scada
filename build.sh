#!/usr/bin/env bash
set -euo pipefail

# Build all deliverables from macOS:
# - macOS packages (arm64 + x86_64) using local toolchain
# - Debian amd64 and arm64 .deb packages (via Docker + QEMU)
# - Windows x86_64 zip (requires nssm.exe provided locally)
# - Windows NSIS installer (Setup.exe) if makensis and nssm.exe present
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
NSSM_PATH="${NSSM_PATH:-$ROOT/packaging/windows/nssm.exe}"

ensure_nssm() {
  if [ ! -f "$NSSM_PATH" ]; then
    echo "⚠️  nssm.exe not found at $NSSM_PATH. Place it there to include in zip/installer."
  fi
}

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
      VERSION=$VERSION make deb"
}

build_windows() {
  ensure_nssm
  build_windows_target x86_64-pc-windows-gnu x86_64
}

build_windows_target() {
  local target="$1"
  local arch="$2"

  echo "==> Building Windows ${arch} (cross, docker + mingw)"
  docker run --rm --platform=linux/amd64 \
    -v "$ROOT":/src -w /src \
    rust:1.94-bullseye \
    bash -lc "set -e; \
      apt-get update; \
      apt-get install -y curl ca-certificates gnupg; \
      curl -fsSL https://deb.nodesource.com/setup_24.x | bash -; \
      apt-get install -y nodejs mingw-w64 zip protobuf-compiler pkg-config; \
      export PATH=\"/usr/local/cargo/bin:\$PATH\"; \
      rustup target add ${target}; \
      make frontend; \
      cargo build --release --target ${target}"

  if [ ! -f "$NSSM_PATH" ]; then
    echo "⚠️  nssm.exe not found at $NSSM_PATH. Place it there to include in the zip."
  fi

  local OUT_DIR="$ROOT/distributions/windows-${arch}"
  mkdir -p "$OUT_DIR"
  cp "$ROOT/target/${target}/release/lirays-scada.exe" "$OUT_DIR/"
  cp "$ROOT/packaging/windows/install.ps1" "$OUT_DIR/"
  cp "$ROOT/packaging/windows/uninstall.ps1" "$OUT_DIR/"
  [ -f "$NSSM_PATH" ] && cp "$NSSM_PATH" "$OUT_DIR/"

  (cd "$ROOT/distributions" && zip -r "lirays-scada-${VERSION}-windows-${arch}.zip" "windows-${arch}")

  # NSIS installer if makensis and nssm are available
  if command -v makensis >/dev/null 2>&1 && [ -f "$NSSM_PATH" ]; then
    echo "==> Building NSIS installer (${arch})"
    cp "$ROOT/packaging/windows/setup.nsi" "$OUT_DIR/"
    (cd "$OUT_DIR" && makensis -DVERSION="$VERSION" -DARCH="${arch}" setup.nsi)
    mv "$OUT_DIR/LiRays-Scada-${VERSION}-${arch}-Setup.exe" "$ROOT/distributions/"
  else
    echo "⚠️  makensis or nssm.exe missing; skipping NSIS installer for ${arch}."
  fi

  rm -rf "$OUT_DIR"
}

build_macos_all() {
  echo "==> Cleaning tree"
  VERSION="$VERSION" make clean

  echo "==> Building macOS arm64"
  VERSION="$VERSION" make mac

  echo "==> Building macOS x86_64 (cross)"
  rustup target add x86_64-apple-darwin >/dev/null 2>&1 || true
  SDKROOT="$(xcrun --sdk macosx --show-sdk-path)" \
    ARCH=x86_64 VERSION="$VERSION" make mac
}

main() {
  mkdir -p "$DIST_DIR"

  # macOS arm64 + x86_64
  build_macos_all

  # Debian packages via Docker (amd64 & arm64)
  run_docker_build linux/amd64
  run_docker_build linux/arm64

  # Windows x86_64 zip (requires nssm.exe pre-provided)
  build_windows

  echo "==> Done. Artifacts in $DIST_DIR"
  ls -lh "$DIST_DIR"
}

main
