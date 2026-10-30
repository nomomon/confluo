# Confluo

Native macOS app (Rust, GPUI 0.2.2) that aggregates pull/merge requests from GitHub, GitLab and Azure DevOps. CLI `gv` shares the engine.

## Map

- `crates/core` domain types, `Provider` trait, capabilities, diff parser. No IO.
- `crates/http` shared per-host rate governor, ETags, request counting (`GV_DEBUG_HTTP=1`).
- `crates/auth` tokens from env, `gh`, `glab`, `az`.
- `crates/provider-{github,gitlab,azure}`, `crates/store` (SQLite), `crates/engine` (facade for UIs).
- `apps/cli` (`gv`), `apps/desktop` (Confluo). `ui/` never imports `features/`, `state/` or the engine.

## Read before changing code

- `docs/design-system.md` tokens, components, motion, layering. Follow it.
- `docs/development.md` dev hooks (`CONFLUO_*`), screenshots, rate limits, gotchas.
- `docs/ux-review.md` known UX issues. `docs/brand.md` name and icon.

## Rules

- Conventional Commits, small steps, push to `main`.
- Never use em or en dashes anywhere. Use hyphens, commas, parentheses.
- Files under ~200 lines. Reuse ui kit components; no ad hoc colors, sizes or durations.
- Machine has 8 GB RAM: one cargo build at a time, `jobs = 4`.
- `cargo clippy --workspace --all-targets` clean and `cargo test --workspace` green before pushing.
- Live writes to providers only on `nomomon/zeron#4`, always reverted. Work orgs are read-only.
- Never put work PR titles, names or code in commits, docs or screenshots.
- Install locally: `scripts/install-macos.sh`.

Be concise in code comments, commits and reports.
