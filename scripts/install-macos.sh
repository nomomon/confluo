#!/usr/bin/env bash
# Build Confluo.app and install it to /Applications, replacing an older copy.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
"$root/scripts/bundle-macos.sh"

dest="/Applications/Confluo.app"
if pgrep -x confluo >/dev/null; then
  echo "Note: Confluo is running; quit and reopen it to use the new version."
fi
rm -rf "$dest"
cp -R "$root/target/release/Confluo.app" "$dest"
touch "$dest"
echo "Installed $dest"
