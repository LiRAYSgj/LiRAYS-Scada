#!/usr/bin/env bash
set -euo pipefail

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This script must be run on macOS." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIST_DIR="$REPO_ROOT/distributions"
ARCH="${ARCH:-$(uname -m)}"       # arm64 or x86_64
VERSION="${VERSION:-0.1.0}"
PKG_ID="com.lirays.scada"
PKG_NAME="lirays-scada-${VERSION}-${ARCH}.pkg"
DMG_NAME="lirays-scada-${VERSION}-${ARCH}.dmg"

if [ "$ARCH" = "x86_64" ]; then
  BIN_PATH="$REPO_ROOT/target/x86_64-apple-darwin/release/lirays-scada"
else
  BIN_PATH="$REPO_ROOT/target/release/lirays-scada"
fi
PLIST_SRC="$REPO_ROOT/packaging/macos/com.lirays.scada.plist"
POSTINSTALL_SRC="$REPO_ROOT/packaging/macos/postinstall"

if [ ! -x "$BIN_PATH" ]; then
  echo "Binary not found at $BIN_PATH. Build the backend first (cargo build --release)." >&2
  exit 1
fi

mkdir -p "$DIST_DIR"

PKGROOT="$(mktemp -d)"
SCRIPTS_DIR="$(mktemp -d)"
DMG_STAGE="$(mktemp -d)"

trap 'rm -rf "$PKGROOT" "$SCRIPTS_DIR" "$DMG_STAGE"' EXIT

echo "Staging payload at $PKGROOT"

/usr/bin/install -d "$PKGROOT/usr/local/bin"
/usr/bin/install -m 755 "$BIN_PATH" "$PKGROOT/usr/local/bin/lirays-scada"

/usr/bin/install -d "$PKGROOT/usr/local/var/lirays-scada/data_dir"
/usr/bin/install -d "$PKGROOT/usr/local/var/log"

/usr/bin/install -d "$PKGROOT/Library/LaunchDaemons"
/usr/bin/install -m 644 "$PLIST_SRC" "$PKGROOT/Library/LaunchDaemons/com.lirays.scada.plist"

/usr/bin/install -m 755 "$POSTINSTALL_SRC" "$SCRIPTS_DIR/postinstall"

echo "Cleaning previous artifacts"
rm -f "$DIST_DIR/$PKG_NAME" "$DIST_DIR/$DMG_NAME"

echo "Building pkg -> $DIST_DIR/$PKG_NAME"
/usr/bin/pkgbuild \
  --root "$PKGROOT" \
  --identifier "$PKG_ID" \
  --version "$VERSION" \
  --scripts "$SCRIPTS_DIR" \
  "$DIST_DIR/$PKG_NAME"

echo "Creating dmg -> $DIST_DIR/$DMG_NAME"
cp "$DIST_DIR/$PKG_NAME" "$DMG_STAGE/"
/usr/bin/hdiutil create -fs HFS+ -volname "LiRAYS-SCADA" -srcfolder "$DMG_STAGE" "$DIST_DIR/$DMG_NAME" >/dev/null

echo "Done."
echo "  PKG: $DIST_DIR/$PKG_NAME"
echo "  DMG: $DIST_DIR/$DMG_NAME"
