#!/usr/bin/env bash
# Build a local Linux x86_64 release artifact matching the GitHub Actions release output.
# Produces:
#   dist/jereide-${VERSION}-linux-x86_64/         (staging directory)
#   dist/jereide-${VERSION}-linux-x86_64.tar.gz   (release archive)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(awk -F'"' '
    /^\[workspace\.package\]$/ { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section && $1 ~ /^version = / { print $2; exit }
' Cargo.toml)"

[ -n "$VERSION" ] || { echo "error: could not read version from Cargo.toml" >&2; exit 1; }

ARCHIVE_BASE="jereide-${VERSION}-linux-x86_64"
DIST_DIR="dist"
STAGE_DIR="$DIST_DIR/$ARCHIVE_BASE"
ARCHIVE="$DIST_DIR/${ARCHIVE_BASE}.tar.gz"

cargo build --release --workspace

rm -rf "$STAGE_DIR" "$ARCHIVE"
mkdir -p "$STAGE_DIR"

cp target/release/jereide "$STAGE_DIR/"
cp -r data "$STAGE_DIR/"
# SDL3 is statically linked via sdl3-sys — nothing to bundle under lib/.
cp resources/linux/com.jeremy.jereide.desktop "$STAGE_DIR/"
# Install the hicolor theme icon under its reverse-DNS app ID.
cp resources/icons/jereide.png "$STAGE_DIR/com.jeremy.jereide.png"

tar -C "$DIST_DIR" -czf "$ARCHIVE" "$ARCHIVE_BASE"

echo "Built archive: $ARCHIVE"
echo "Staging dir:   $STAGE_DIR"
