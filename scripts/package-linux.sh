#!/usr/bin/env bash
# Package a release build of Confluo for Linux into dist/:
#   Confluo-X.Y.Z-<arch>.AppImage, confluo_X.Y.Z-1_<debarch>.deb, Confluo-X.Y.Z-linux-<arch>.tar.gz
# Expects `cargo build --release -p gv-desktop` done and cargo-deb installed. Runs on Ubuntu CI.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
v="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
arch="$(uname -m)"
bin=target/release/confluo
mkdir -p dist
work="$(mktemp -d)"

# tar.gz: binary, desktop entry, icon.
t="$work/Confluo-$v"
mkdir -p "$t"
cp "$bin" packaging/linux/confluo.desktop packaging/linux/confluo.png LICENSE "$t/"
tar -C "$work" -czf "dist/Confluo-$v-linux-$arch.tar.gz" "Confluo-$v"

# .deb (assets in apps/desktop/Cargo.toml).
cargo deb -p gv-desktop --no-build --no-strip --output dist/

# AppImage via appimagetool.
ad="$work/AppDir"
mkdir -p "$ad/usr/bin" "$ad/usr/share/applications" "$ad/usr/share/icons/hicolor/512x512/apps"
cp "$bin" "$ad/usr/bin/confluo"
cp packaging/linux/confluo.desktop "$ad/confluo.desktop"
cp packaging/linux/confluo.desktop "$ad/usr/share/applications/"
cp packaging/linux/confluo.png "$ad/confluo.png"
cp packaging/linux/confluo.png "$ad/usr/share/icons/hicolor/512x512/apps/"
cat > "$ad/AppRun" <<'EOF'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/confluo" "$@"
EOF
chmod +x "$ad/AppRun"
tool="$work/appimagetool"
curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
chmod +x "$tool"
ARCH="$arch" APPIMAGE_EXTRACT_AND_RUN=1 "$tool" "$ad" "dist/Confluo-$v-$arch.AppImage"

rm -rf "$work"
ls -la dist
