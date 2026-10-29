# Development

```sh
cargo run -p gv-desktop          # desktop app (binary: confluo)
cargo run -p gv-cli -- list      # CLI
cargo test --workspace
scripts/install-macos.sh         # build, bundle and copy to /Applications
```

Dev-only env: `CONFLUO_OPEN=<pr url>`, `CONFLUO_TAB=code|activity`, `CONFLUO_SELECT=path:start-end`, `CONFLUO_REVEAL=<thread id>`.
Data: `~/Library/Application Support/Confluo/` (override cache with `GV_DB`; avatars in `avatars/`).
`GV_DEBUG_HTTP=1 gv list` prints request counts per host and endpoint after the refresh.

## Rate limits

Every provider request goes through `crates/http`: one governor per host (token bucket 5 req/s,
burst 10, 4 in flight), ETag revalidation for GETs, and the `x-ratelimit-*` / `ratelimit-*` /
`Retry-After` headers. Low remaining budget slows a host to 0.5 req/s; a 429 or secondary-limit
403 pauses it (Retry-After, reset time, or 60s doubling backoff with jitter), retries once only
if the pause is under 10s, and otherwise returns `RateLimited` without sending anything more.
The engine skips paused hosts, reports `paused_until`, coalesces refreshes of the same target,
re-resolves group repo lists every 6h, and fetches GitLab reviewer states and pipelines only for
MRs whose `updated_at` moved (or whose checks were running), never for hidden bot MRs.
The app refreshes every 5 min, on focus only after 2 min, and debounces manual refreshes (5s).

## Bot PRs

`BotFilter` (core) hides PRs by author type (GitHub `Bot`, `[bot]` logins), author patterns
(`renovate`, `*-bot`, `project_*_bot*`, ...) and branch prefixes (`renovate/`, `dependabot/`).
Configured in Settings (`hide_bots`, `hidden_authors`, `hidden_branch_prefixes` in
`settings.json`, also read by the CLI); `gv list --include-bots` shows them.

## Layout

- `crates/core` domain types, `Provider` trait, diff parser (no IO)
- `crates/auth` tokens from env, `gh`, `glab`, `az`
- `crates/discovery` find repos in a folder, classify remotes
- `crates/provider-{github,gitlab,azure}`
- `crates/store` SQLite cache
- `crates/http` governed HTTP: per-host limits, ETags, request stats
- `crates/engine` facade for UIs
- `apps/cli` the `gv` binary
- `apps/desktop` the Confluo app (GPUI)

See also `design-system.md` and `brand.md`.
