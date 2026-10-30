#!/usr/bin/env bash
# Linux smoke test: run Confluo in demo mode on a virtual X display for ~10 s, fail if it exits
# (panic, crash) before then, and save a screenshot plus its output to <out>/.
# Usage: scripts/smoke-linux.sh <binary> <out-dir>. Needs xvfb, imagemagick, mesa-vulkan-drivers.
set -uo pipefail
bin="$1"
out="$2"
mkdir -p "$out"
export DISPLAY=:99
Xvfb :99 -screen 0 1440x900x24 >/dev/null 2>&1 &
xvfb=$!
sleep 2
CONFLUO_DEMO=1 CONFLUO_NO_REFRESH=1 RUST_BACKTRACE=1 XDG_STATE_HOME="$out" "$bin" >"$out/stdout.log" 2>"$out/stderr.log" &
app=$!
sleep 10
import -window root "$out/confluo-linux.png" || echo "screenshot failed"
status=0
if kill -0 "$app" 2>/dev/null; then
    echo "Confluo still running after 10 s: ok"
    kill "$app"
else
    wait "$app"
    echo "Confluo exited early with status $?"
    status=1
fi
kill "$xvfb" 2>/dev/null
tail -n 50 "$out/stderr.log"
ls "$out"/confluo/crash-*.log 2>/dev/null && status=1
exit $status
