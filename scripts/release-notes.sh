#!/usr/bin/env bash
# Release notes from Conventional Commits between the previous tag and <tag> (default HEAD).
# Usage: scripts/release-notes.sh v0.2.0
set -euo pipefail
tag="${1:-HEAD}"
prev="$(git describe --tags --abbrev=0 "$tag^" 2>/dev/null || true)"
range="${prev:+$prev..}$tag"
log="$(git log --no-merges --format='%s' "$range")"

section() { # title, type regex
  local lines
  lines="$(printf '%s\n' "$log" | grep -E "^($2)(\([^)]*\))?!?: " | sed -E 's/^[a-z]+(\(([^)]*)\))?!?: (.*)$/- \3 (\2)/; s/ \(\)$//' || true)"
  [ -n "$lines" ] && printf '### %s\n\n%s\n\n' "$1" "$lines"
  return 0
}

section "Features" "feat"
section "Fixes" "fix"
section "Performance" "perf"
section "Other changes" "refactor|build|ci|docs|chore|style|test"
[ -n "$prev" ] && echo "**Full changelog**: https://github.com/nomomon/confluo/compare/$prev...$tag"
exit 0
