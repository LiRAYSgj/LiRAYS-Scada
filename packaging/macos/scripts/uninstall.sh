#!/bin/bash
# Uninstall LiRAYS Scada completely (service, app, configs, data, logs, receipts, staging)
# Usage: sudo ./uninstall.sh

set -euo pipefail

APP="/Applications/LiRAYS Scada.app"
PLIST="/Library/LaunchDaemons/com.lirays.scada.plist"
BIN="/usr/local/bin/lirays-scada"
SUPPORT_NEW="/Library/LiRAYS-Scada"
SUPPORT_OLD="/Library/Application Support/LiRAYS-Scada"
LOGS="/Library/Logs/LiRAYS-Scada"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
STAGING1="$REPO_ROOT/packaging/macos/payload"
STAGING2="$REPO_ROOT/packaging/macos/build"

echo "Stopping service..."
launchctl bootout system/com.lirays.scada 2>/dev/null || true
launchctl disable system/com.lirays.scada 2>/dev/null || true

echo "Removing launchd plist..."
rm -f "$PLIST"

echo "Removing binary and app..."
rm -f "$BIN"
rm -rf "$APP"

echo "Removing configs/data/logs..."
rm -rf "$SUPPORT_NEW" "$SUPPORT_OLD" "$LOGS"

echo "Forgetting pkg receipt (if any)..."
pkgutil --forget com.lirays.scada 2>/dev/null || true

echo "Cleaning local staging (if present)..."
rm -rf "$STAGING1" "$STAGING2"

echo "Uninstall complete."
