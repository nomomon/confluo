#!/usr/bin/env bash
# Linux smoke test: run Confluo in demo mode on a virtual X display (Xvfb + lavapipe software Vulkan)
# for ~15 s; fail if it exits early, crashes, or the screenshot is blank. Output goes to <out>/.
# Usage: scripts/smoke-linux.sh <binary> <out-dir>. Needs xvfb, x11-utils, x11-apps, imagemagick,
# mesa-vulkan-drivers, vulkan-tools.
set -uo pipefail
bin="$1"
out="$2"
mkdir -p "$out"
export DISPLAY=:99
icd=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json 2>/dev/null | head -n1)
[ -n "$icd" ] && export VK_ICD_FILENAMES="$icd"
Xvfb :99 -screen 0 1440x900x24 >/dev/null 2>&1 &
xvfb=$!
sleep 2
vulkaninfo --summary >"$out/vulkaninfo.txt" 2>&1 || echo "vulkaninfo failed"
grep -E "deviceName|driverName" "$out/vulkaninfo.txt" || true
CONFLUO_DEMO=1 CONFLUO_NO_REFRESH=1 RUST_BACKTRACE=1 RUST_LOG="${RUST_LOG:-info}" XDG_STATE_HOME="$out" \
    "$bin" >"$out/stdout.log" 2>"$out/stderr.log" &
app=$!
sleep 15
xwininfo -root -tree >"$out/windows.txt" 2>&1 || true
xwd -root -silent | convert xwd:- "$out/confluo-linux.png" || echo "screenshot failed"
win=$(xwininfo -root -tree | awk '/"Confluo"/ {print $1; exit}')
[ -n "$win" ] && { import -window "$win" "$out/window-import.png" || true; }
status=0
if kill -0 "$app" 2>/dev/null; then
    echo "Confluo still running after 15 s: ok"
    kill "$app"
else
    wait "$app"
    echo "Confluo exited early with status $?"
    status=1
fi
kill "$xvfb" 2>/dev/null
tail -n 50 "$out/stderr.log"
ls "$out"/confluo/crash-*.log 2>/dev/null && status=1
# A rendered window has many colors and real contrast; a black or flat frame has neither.
colors=$(identify -format %k "$out/confluo-linux.png" 2>/dev/null || echo 0)
sd=$(identify -format '%[fx:int(standard_deviation*1000)]' "$out/confluo-linux.png" 2>/dev/null || echo 0)
echo "screenshot: $colors colors, stddev x1000 = $sd"
if [ "$colors" -lt 50 ] || [ "$sd" -lt 20 ]; then
    echo "::error::screenshot is blank, the window did not render"
    status=1
fi
exit $status
