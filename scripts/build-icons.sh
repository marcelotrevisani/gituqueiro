#!/usr/bin/env bash
set -euo pipefail

# Generate all platform icon formats from the SVG source.
# Requires: rsvg-convert (librsvg), sips (macOS), iconutil (macOS)
# For Windows .ico: requires ImageMagick (convert) or falls back to bundled PNGs.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SVG="$ROOT_DIR/icons/icon.svg"
OUT="$ROOT_DIR/icons"

mkdir -p "$OUT"

echo "==> Generating PNGs..."
for SIZE in 16 32 48 64 128 256 512 1024; do
    rsvg-convert -w "$SIZE" -h "$SIZE" "$SVG" -o "$OUT/icon-${SIZE}x${SIZE}.png"
    echo "    ${SIZE}x${SIZE}"
done

# Copy the 1024 as the canonical icon.png
cp "$OUT/icon-1024x1024.png" "$OUT/icon.png"

# --- macOS .icns ---
if command -v iconutil &>/dev/null; then
    echo "==> Building macOS .icns..."
    ICONSET="$OUT/gituqueiro.iconset"
    mkdir -p "$ICONSET"

    for SIZE in 16 32 128 256 512; do
        DOUBLE=$((SIZE * 2))
        cp "$OUT/icon-${SIZE}x${SIZE}.png"   "$ICONSET/icon_${SIZE}x${SIZE}.png"
        cp "$OUT/icon-${DOUBLE}x${DOUBLE}.png" "$ICONSET/icon_${SIZE}x${SIZE}@2x.png"
    done

    iconutil -c icns "$ICONSET" -o "$OUT/icon.icns"
    rm -rf "$ICONSET"
    echo "    icon.icns"
fi

# --- Windows .ico ---
ICO_SIZES=(
    "$OUT/icon-16x16.png"
    "$OUT/icon-32x32.png"
    "$OUT/icon-48x48.png"
    "$OUT/icon-64x64.png"
    "$OUT/icon-128x128.png"
    "$OUT/icon-256x256.png"
)
if command -v magick &>/dev/null; then
    echo "==> Building Windows .ico (ImageMagick 7)..."
    magick "${ICO_SIZES[@]}" "$OUT/icon.ico"
    echo "    icon.ico"
elif command -v convert &>/dev/null; then
    echo "==> Building Windows .ico (ImageMagick)..."
    convert "${ICO_SIZES[@]}" "$OUT/icon.ico"
    echo "    icon.ico"
else
    echo "==> Skipping .ico (install ImageMagick for Windows icon)"
fi

echo "==> Done! Icons in $OUT/"
ls -la "$OUT"/icon.{svg,png,icns,ico} 2>/dev/null || true
