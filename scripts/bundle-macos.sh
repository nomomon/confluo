#!/usr/bin/env bash
# Build Confluo in release mode and wrap it in target/release/Confluo.app.
# Usage: scripts/bundle-macos.sh   (CARGO_BUILD_JOBS limits parallelism, default 4)
# CONFLUO_BINARY=path skips the build and bundles that binary (release-macos.sh passes a universal one).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

jobs="${CARGO_BUILD_JOBS:-4}"
binary="${CONFLUO_BINARY:-}"
if [ -z "$binary" ]; then
  cargo build --release -p gv-desktop -j "$jobs"
  binary=target/release/confluo
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
app="${CONFLUO_APP_DIR:-target/release}/Confluo.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
mkdir -p "$(dirname "$app")"
cp "$binary" "$app/Contents/MacOS/confluo"
cp assets/brand/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Confluo</string>
  <key>CFBundleDisplayName</key><string>Confluo</string>
  <key>CFBundleIdentifier</key><string>com.nomomon.confluo</string>
  <key>CFBundleExecutable</key><string>confluo</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --deep -s - "$app"
echo "Built $root/$app (version $version)"
