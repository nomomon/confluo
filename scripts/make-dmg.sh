#!/usr/bin/env bash
# Package Confluo.app into a branded Confluo-X.Y.Z.dmg (drag to Applications).
# Usage: scripts/make-dmg.sh [path/to/Confluo.app] [out dir]
# Uses `create-dmg` (brew install create-dmg) when present, else hdiutil plus a Finder AppleScript.
# Signs the dmg when APPLE_SIGNING_IDENTITY is set.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
app="${1:-$root/target/release/Confluo.app}"
out="${2:-$root/target/release}"
[ -d "$app" ] || { echo "error: $app not found (run scripts/bundle-macos.sh)" >&2; exit 1; }
version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")"
dmg="$out/Confluo-$version.dmg"
name="Confluo $version"

# Background 660x400; slot centers match scripts/brand/generate_dmg_background.py. The window
# is taller than the background by the Finder title bar, toolbar and status bar.
win_w=660 win_h=495 icon=128 app_x=170 apps_x=490 slot_y=190

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# One TIFF holding the 1x and 2x backgrounds, so Retina windows stay sharp.
tiffutil -cathidpicheck "$root/assets/brand/dmg-background.png" "$root/assets/brand/dmg-background@2x.png" -out "$work/background.tiff" >/dev/null 2>&1
rm -f "$dmg"
mkdir -p "$out"

if command -v create-dmg >/dev/null; then
  mkdir "$work/src"
  ditto "$app" "$work/src/Confluo.app"
  create-dmg \
    --volname "$name" \
    --volicon "$root/assets/brand/AppIcon.icns" \
    --background "$work/background.tiff" \
    --window-pos 200 120 \
    --window-size "$win_w" "$win_h" \
    --icon-size "$icon" \
    --text-size 13 \
    --icon "Confluo.app" "$app_x" "$slot_y" \
    --hide-extension "Confluo.app" \
    --app-drop-link "$apps_x" "$slot_y" \
    --no-internet-enable \
    "$dmg" "$work/src" >/dev/null
else
  stage="$work/stage"
  mkdir -p "$stage/.background"
  ditto "$app" "$stage/Confluo.app"
  ln -s /Applications "$stage/Applications"
  cp "$work/background.tiff" "$stage/.background/background.tiff"
  hdiutil create -quiet -srcfolder "$stage" -volname "$name" -fs HFS+ -format UDRW -ov "$work/rw.dmg"
  dev="$(hdiutil attach -readwrite -noverify -noautoopen "$work/rw.dmg" | awk '/Apple_HFS/ {print $1}')"
  sleep 1
  osascript <<APPLESCRIPT
tell application "Finder"
  tell disk "$name"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {200, 120, $((200 + win_w)), $((120 + win_h))}
    set opts to the icon view options of container window
    set arrangement of opts to not arranged
    set icon size of opts to $icon
    set background picture of opts to file ".background:background.tiff"
    set position of item "Confluo.app" of container window to {$app_x, $slot_y}
    set position of item "Applications" of container window to {$apps_x, $slot_y}
    update without registering applications
    delay 1
    close
  end tell
end tell
APPLESCRIPT
  sync
  hdiutil detach -quiet "$dev"
  hdiutil convert -quiet "$work/rw.dmg" -format UDZO -imagekey zlib-level=9 -o "$dmg"
fi

if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --timestamp -s "$APPLE_SIGNING_IDENTITY" "$dmg"
else
  echo "warning: APPLE_SIGNING_IDENTITY not set; the dmg is unsigned" >&2
fi
echo "Built $dmg"
