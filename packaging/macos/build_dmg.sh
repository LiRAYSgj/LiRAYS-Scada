#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

ARCH="${ARCH:-$(uname -m)}"
VERSION="${VERSION:-0.0.0}"
APP_NAME="LiRays SCADA"
APP_BUNDLE_NAME="LiRays-Scada.app"
BIN_NAME="lirays-scada"
VOLUME_NAME="LiRays SCADA"
ICON_SRC="$ROOT_DIR/frontend/static/android-chrome-512x512.png"
ICON_NAME="icon.icns"
DEFAULT_CONFIG="$ROOT_DIR/settings-default.yaml"

DIST_DIR="$ROOT_DIR/distributions"
STAGING_DIR="$SCRIPT_DIR/build-${ARCH}"
APP_DIR="$STAGING_DIR/$APP_BUNDLE_NAME"
DMG_PATH="$DIST_DIR/lirays-scada-${VERSION}-mac-${ARCH}.dmg"

cleanup() {
  rm -rf "$STAGING_DIR"
}
trap cleanup EXIT

mkdir -p "$DIST_DIR"
rm -f "$DMG_PATH"
rm -rf "$STAGING_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

build_icns() {
  if [ ! -f "$ICON_SRC" ]; then
    echo "⚠️  Icon source not found at $ICON_SRC; skipping icon generation."
    return
  fi
  if ! command -v sips >/dev/null || ! command -v iconutil >/dev/null; then
    echo "⚠️  sips/iconutil not available; skipping icon generation."
    return
  fi

  local iconset="$STAGING_DIR/icon.iconset"
  rm -rf "$iconset"
  mkdir -p "$iconset"

  for size in 16 32 64 128 256 512 1024; do
    local base="$iconset/icon_${size}x${size}.png"
    sips -z "$size" "$size" "$ICON_SRC" --out "$base" >/dev/null
    if [ "$size" -ne 1024 ]; then
      local retina=$((size * 2))
      sips -z "$retina" "$retina" "$ICON_SRC" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    fi
  done

  iconutil -c icns "$iconset" -o "$APP_DIR/Contents/Resources/$ICON_NAME" >/dev/null
}

build_icns

choose_binary() {
  case "$ARCH" in
    arm64|aarch64)
      candidates=(
        "$ROOT_DIR/target/aarch64-apple-darwin/release/$BIN_NAME"
        "$ROOT_DIR/target/arm64-apple-darwin/release/$BIN_NAME"
        "$ROOT_DIR/target/release/$BIN_NAME"
      )
      ;;
    x86_64)
      candidates=(
        "$ROOT_DIR/target/x86_64-apple-darwin/release/$BIN_NAME"
        "$ROOT_DIR/target/release/$BIN_NAME"
      )
      ;;
    *)
      echo "Unsupported ARCH: $ARCH" >&2
      exit 2
      ;;
  esac

  for bin in "${candidates[@]}"; do
    if [ -f "$bin" ]; then
      echo "$bin"
      return 0
    fi
  done

  echo "Binary $BIN_NAME not found for arch $ARCH. Build it first (make mac-local/mac-all)." >&2
  exit 1
}

BIN_PATH="$(choose_binary)"
install -m 755 "$BIN_PATH" "$APP_DIR/Contents/MacOS/$BIN_NAME"

cat >"$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>${BIN_NAME}</string>
  <key>CFBundleIdentifier</key><string>com.lirays.scada</string>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleIconFile</key><string>${ICON_NAME}</string>
</dict>
</plist>
EOF

# Ship default settings alongside the app bundle so first run can copy to Application Support
if [ -f "$DEFAULT_CONFIG" ]; then
  cp "$DEFAULT_CONFIG" "$APP_DIR/Contents/Resources/settings.yaml"
fi

DMG_ROOT="$STAGING_DIR/dmg-root"
mkdir -p "$DMG_ROOT"
cp -R "$APP_DIR" "$DMG_ROOT/"
ln -sf /Applications "$DMG_ROOT/Applications"

hdiutil create \
  -volname "$VOLUME_NAME" \
  -srcfolder "$DMG_ROOT" \
  -fs HFS+ \
  -format UDZO \
  -ov \
  "$DMG_PATH" >/dev/null

echo "✅ Built DMG: $DMG_PATH"
