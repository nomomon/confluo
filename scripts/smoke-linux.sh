#!/usr/bin/env bash
# Linux smoke test: run Confluo in demo mode under a headless Wayland compositor (sway, pixman
# renderer) with lavapipe software Vulkan for ~15 s; fail if it exits early, crashes, or the
# screenshot is blank. Output goes to <out>/. Xvfb is not used: gpui's X11 backend never presents
# a frame there (renderer initializes, window maps, stays black).
# Usage: scripts/smoke-linux.sh <binary> <out-dir>. Needs sway, grim, imagemagick,
# mesa-vulkan-drivers, vulkan-tools.
set -uo pipefail
bin="$1"
out="$(realpath -m "$2")"
mkdir -p "$out"
icd=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json 2>/dev/null | head -n1)
[ -n "$icd" ] && export VK_ICD_FILENAMES="$icd"
export XDG_RUNTIME_DIR="$(mktemp -d)"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
printf 'output HEADLESS-1 resolution 1440x900\ndefault_border none\n' >"$out/sway.conf"
sway -c "$out/sway.conf" >"$out/sway.log" 2>&1 &
sway_pid=$!
for _ in $(seq 50); do
    WAYLAND_DISPLAY=$(ls "$XDG_RUNTIME_DIR" | grep -m1 '^wayland-[0-9]*$') && break
    sleep 0.2
done
export WAYLAND_DISPLAY
unset DISPLAY
echo "compositor socket: ${WAYLAND_DISPLAY:-none}"
vulkaninfo --summary >"$out/vulkaninfo.txt" 2>&1 || echo "vulkaninfo failed"
grep -E "deviceName|driverName" "$out/vulkaninfo.txt" || true
CONFLUO_DEMO=1 CONFLUO_NO_REFRESH=1 RUST_BACKTRACE=1 RUST_LOG="${RUST_LOG:-info}" XDG_STATE_HOME="$out" \
    "$bin" >"$out/stdout.log" 2>"$out/stderr.log" &
app=$!
sleep 15
grim "$out/confluo-linux.png" || echo "screenshot failed"
status=0
if kill -0 "$app" 2>/dev/null; then
    echo "Confluo still running after 15 s: ok"
    kill "$app"
else
    wait "$app"
    echo "Confluo exited early with status $?"
    status=1
fi
kill "$sway_pid" 2>/dev/null
grep -vE "naga::back" "$out/stderr.log" | tail -n 30
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
