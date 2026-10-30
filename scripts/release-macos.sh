#!/usr/bin/env bash
# Build, sign, notarize and package a Confluo release into dist/:
#   Confluo-X.Y.Z-macos.zip (the app, for the self-updater), Confluo-X.Y.Z.dmg, SHA256SUMS.
#
# Architectures: universal (arm64 + x86_64 via lipo) on CI or with UNIVERSAL=1; otherwise the host
# architecture only (an x86_64 cross build is heavy for an 8 GB machine; CI builds it).
#
# Signing and notarization use these env vars (see docs/releasing.md):
#   APPLE_SIGNING_IDENTITY   "Developer ID Application: Name (TEAMID)"
#   APPLE_API_KEY_PATH, APPLE_API_KEY_ID, APPLE_API_ISSUER   App Store Connect API key for notarytool
# Without them the app is ad-hoc signed and not notarized (fine for local testing only).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
dist="$root/dist"
jobs="${CARGO_BUILD_JOBS:-4}"
identity="${APPLE_SIGNING_IDENTITY:-}"
notarize=0
if [ -n "$identity" ] && [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY_ID:-}" ] && [ -n "${APPLE_API_ISSUER:-}" ]; then
  notarize=1
fi
if [ -z "$identity" ]; then
  echo "WARNING: APPLE_SIGNING_IDENTITY is not set: ad-hoc signing, no notarization." >&2
  echo "WARNING: Gatekeeper will block this build on other Macs. Use it for local testing only." >&2
elif [ "$notarize" = 0 ]; then
  echo "WARNING: App Store Connect API key not set: signed but NOT notarized." >&2
fi

host="$(uname -m | sed 's/arm64/aarch64/')"
if [ "${CI:-}" = "true" ] || [ "${UNIVERSAL:-0}" = 1 ]; then archs="aarch64 x86_64"; else archs="$host"; fi

bins=()
for a in $archs; do
  target="$a-apple-darwin"
  rustup target list --installed | grep -qx "$target" || rustup target add "$target"
  echo "==> cargo build --release --target $target"
  cargo build --release -p gv-desktop --target "$target" -j "$jobs"
  bins+=("target/$target/release/confluo")
done
mkdir -p target/universal
bin="target/universal/confluo"
if [ "${#bins[@]}" -gt 1 ]; then lipo -create -output "$bin" "${bins[@]}"; else cp "${bins[0]}" "$bin"; fi
lipo -info "$bin"

rm -rf "$dist" && mkdir -p "$dist"
CONFLUO_BINARY="$bin" CONFLUO_APP_DIR="$dist" scripts/bundle-macos.sh >/dev/null
app="$dist/Confluo.app"

notarize_file() {
  [ "$notarize" = 1 ] || return 0
  echo "==> notarytool submit $(basename "$1")"
  xcrun notarytool submit "$1" --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER" --wait --timeout 30m
}

if [ -n "$identity" ]; then
  echo "==> codesign ($identity)"
  codesign --force --options runtime --timestamp --entitlements scripts/macos/Confluo.entitlements -s "$identity" "$app/Contents/MacOS/confluo"
  codesign --force --options runtime --timestamp --entitlements scripts/macos/Confluo.entitlements -s "$identity" "$app"
else
  codesign --force --options runtime --entitlements scripts/macos/Confluo.entitlements -s - "$app"
fi
codesign --verify --deep --strict "$app"

zip="$dist/Confluo-$version-macos.zip"
if [ "$notarize" = 1 ]; then
  ditto -c -k --keepParent "$app" "$dist/notarize.zip"
  notarize_file "$dist/notarize.zip"
  rm "$dist/notarize.zip"
  xcrun stapler staple "$app"
fi
ditto -c -k --keepParent "$app" "$zip"

scripts/make-dmg.sh "$app" "$dist"
dmg="$dist/Confluo-$version.dmg"
if [ "$notarize" = 1 ]; then
  notarize_file "$dmg"
  xcrun stapler staple "$dmg"
fi

(cd "$dist" && shasum -a 256 "$(basename "$zip")" "$(basename "$dmg")" > SHA256SUMS)
rm -rf "$app"
echo "==> dist/"
ls -lh "$dist"
cat "$dist/SHA256SUMS"
