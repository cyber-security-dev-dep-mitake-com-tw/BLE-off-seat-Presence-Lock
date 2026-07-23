#!/usr/bin/env bash
# Build macOS .app, .dmg, and .pkg for BLE Presence Lock.
# Usage: packaging/macos/build.sh [version]
# Env:
#   TARGET_DIR   – cargo target dir (default: target)
#   DIST_DIR     – output directory (default: dist/macos)
#   SKIP_BUILD   – if 1, reuse existing release binaries
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"
TARGET_DIR="${TARGET_DIR:-target}"
DIST_DIR="${DIST_DIR:-dist/macos}"
APP_NAME="BLE Presence Lock"
BINARY_NAME="ble-presence-lock"
BUNDLE_ID="com.blepresencelock.app"
PKG_ID="${BUNDLE_ID}.pkg"

ARM_TARGET="aarch64-apple-darwin"
X86_TARGET="x86_64-apple-darwin"

echo "==> Building macOS packages for v${VERSION}"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  rustup target add "$ARM_TARGET" "$X86_TARGET"
  echo "==> cargo build --release (${ARM_TARGET})"
  cargo build --release --target "$ARM_TARGET"
  echo "==> cargo build --release (${X86_TARGET})"
  cargo build --release --target "$X86_TARGET"
fi

ARM_BIN="${TARGET_DIR}/${ARM_TARGET}/release/${BINARY_NAME}"
X86_BIN="${TARGET_DIR}/${X86_TARGET}/release/${BINARY_NAME}"

if [[ ! -f "$ARM_BIN" || ! -f "$X86_BIN" ]]; then
  echo "error: missing release binaries:" >&2
  echo "  $ARM_BIN" >&2
  echo "  $X86_BIN" >&2
  exit 1
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

UNIVERSAL="${WORKDIR}/${BINARY_NAME}"
echo "==> Creating universal binary"
lipo -create -output "$UNIVERSAL" "$ARM_BIN" "$X86_BIN"
chmod +x "$UNIVERSAL"

# --- .app bundle ---
APP_DIR="${WORKDIR}/${APP_NAME}.app"
CONTENTS="${APP_DIR}/Contents"
mkdir -p "${CONTENTS}/MacOS" "${CONTENTS}/Resources"

sed "s/APP_VERSION/${VERSION}/g" packaging/macos/Info.plist > "${CONTENTS}/Info.plist"
cp "$UNIVERSAL" "${CONTENTS}/MacOS/${BINARY_NAME}"
cp config.example.yaml "${CONTENTS}/Resources/config.example.yaml"

# Default config next to binary so --config default works when launched from app.
cp config.example.yaml "${CONTENTS}/MacOS/config.yaml"

mkdir -p "$DIST_DIR"
APP_OUT="${DIST_DIR}/${APP_NAME}.app"
rm -rf "$APP_OUT"
cp -R "$APP_DIR" "$APP_OUT"
echo "==> App bundle: $APP_OUT"

# --- DMG ---
DMG_STAGING="${WORKDIR}/dmg"
mkdir -p "$DMG_STAGING"
cp -R "$APP_DIR" "$DMG_STAGING/"
cp config.example.yaml "$DMG_STAGING/config.example.yaml"
ln -s /Applications "$DMG_STAGING/Applications"

DMG_OUT="${DIST_DIR}/${BINARY_NAME}-${VERSION}-universal.dmg"
rm -f "$DMG_OUT"
hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "$DMG_STAGING" \
  -ov -format UDZO \
  "$DMG_OUT"
echo "==> DMG: $DMG_OUT"

# --- PKG (installs .app into /Applications) ---
PKG_ROOT="${WORKDIR}/pkgroot"
mkdir -p "${PKG_ROOT}/Applications"
cp -R "$APP_DIR" "${PKG_ROOT}/Applications/"

PKG_OUT="${DIST_DIR}/${BINARY_NAME}-${VERSION}-universal.pkg"
rm -f "$PKG_OUT"
pkgbuild \
  --root "$PKG_ROOT" \
  --identifier "$PKG_ID" \
  --version "$VERSION" \
  --install-location "/" \
  "$PKG_OUT"
echo "==> PKG: $PKG_OUT"

# Checksums
(
  cd "$DIST_DIR"
  shasum -a 256 \
    "$(basename "$DMG_OUT")" \
    "$(basename "$PKG_OUT")" \
    > "${BINARY_NAME}-${VERSION}-universal.sha256"
)

echo "==> Done. Artifacts in ${DIST_DIR}/"
ls -lh "$DIST_DIR"
