#!/usr/bin/env bash
set -euo pipefail

CONFIG="${1:-release}"
VERSION="${2:-dev}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
APP_NAME="Gituqueiro"
BUNDLE_ID="com.marcelotrevisani.gituqueiro"

BUILD_DIR="$ROOT_DIR/build"
APP_DIR="$BUILD_DIR/${APP_NAME}.app"

# Build
if [ "$CONFIG" = "release" ]; then
    cargo build --release
    BINARY="$ROOT_DIR/target/release/gituqueiro"
else
    cargo build
    BINARY="$ROOT_DIR/target/debug/gituqueiro"
fi

# Create .app bundle structure
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp "$BINARY" "$APP_DIR/Contents/MacOS/gituqueiro"

# Copy icon
if [ -f "$ROOT_DIR/icons/icon.icns" ]; then
    cp "$ROOT_DIR/icons/icon.icns" "$APP_DIR/Contents/Resources/gituqueiro.icns"
fi

# Create Info.plist
cat > "$APP_DIR/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>gituqueiro</string>
    <key>CFBundleIconFile</key>
    <string>gituqueiro</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
</dict>
</plist>
PLIST

echo "Built: $APP_DIR"
