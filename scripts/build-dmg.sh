#!/usr/bin/env bash
set -euo pipefail

# Builds a DMG containing Gituqueiro.app with an Applications symlink
#
# Usage: ./scripts/build-dmg.sh [TAG]
#   TAG defaults to "v0.0.0-dev"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"
TAG="${1:-v0.0.0-dev}"
VERSION="${TAG#v}"
APP_NAME="Gituqueiro"
DMG_NAME="gituqueiro-${TAG}-macos.dmg"
DMG_PATH="$BUILD_DIR/$DMG_NAME"
APP_DIR="$BUILD_DIR/${APP_NAME}.app"

# Build the .app bundle
"$SCRIPT_DIR/build-app.sh" release "$VERSION"

if [ ! -d "$APP_DIR" ]; then
    echo "ERROR: ${APP_NAME}.app not found at $APP_DIR"
    exit 1
fi

echo "Creating DMG..."

# Prepare staging directory
DMG_STAGING="$BUILD_DIR/dmg-staging"
rm -rf "$DMG_STAGING"
mkdir -p "$DMG_STAGING"
cp -R "$APP_DIR" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

# Create DMG
rm -f "$DMG_PATH"
hdiutil create \
    -volname "$APP_NAME $VERSION" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$DMG_PATH"

# Cleanup staging
rm -rf "$DMG_STAGING"

echo "Created: $DMG_PATH"
echo "SHA256: $(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
