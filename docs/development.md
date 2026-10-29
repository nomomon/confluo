# Development

```sh
cargo run -p gv-desktop          # desktop app (binary: confluo)
cargo run -p gv-cli -- list      # CLI
cargo test --workspace
scripts/install-macos.sh         # build, bundle and copy to /Applications
```

Dev-only env: `CONFLUO_OPEN=<pr url>`, `CONFLUO_TAB=code|activity`, `CONFLUO_SELECT=path:start-end`, `CONFLUO_REVEAL=<thread id>`.
Data: `~/Library/Application Support/Confluo/` (override cache with `GV_DB`).

## Layout

- `crates/core` domain types, `Provider` trait, diff parser (no IO)
- `crates/auth` tokens from env, `gh`, `glab`, `az`
- `crates/discovery` find repos in a folder, classify remotes
- `crates/provider-{github,gitlab,azure}`
- `crates/store` SQLite cache
- `crates/engine` facade for UIs
- `apps/cli` the `gv` binary
- `apps/desktop` the Confluo app (GPUI)

See also `design-system.md` and `brand.md`.
