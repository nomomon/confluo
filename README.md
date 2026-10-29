# Confluo

One place for PRs and MRs across GitHub, GitLab and Azure DevOps: a native macOS app (GPUI) plus the `gv` CLI.

## Install (macOS)

```sh
scripts/install-macos.sh
```

Builds a release binary, bundles `target/release/Confluo.app` (ad-hoc signed, via `scripts/bundle-macos.sh`)
and copies it to `/Applications/Confluo.app`, replacing an older copy. Set `CARGO_BUILD_JOBS=2` on low-RAM machines.
Credentials come from `gh`, `glab` and `az`; they are found in `$PATH` plus `/opt/homebrew/bin`, `/usr/local/bin`
and `~/.local/bin`, so a Finder or Dock launch works too.

## Development

```sh
cargo run -p gv-desktop          # desktop app (binary: confluo)
cargo run -p gv-cli -- list      # CLI
cargo test --workspace
```

## Layout

- `crates/core` domain types, `Provider` trait, needs-you logic (no IO)
- `crates/auth` tokens from env (`GV_TOKEN_<HOST>`, `GH_TOKEN`, `GITLAB_TOKEN`, `AZURE_DEVOPS_EXT_PAT`), `gh`, `glab`, `az`
- `crates/discovery` find git repos in a folder and classify remotes
- `crates/provider-github`, `crates/provider-gitlab`, `crates/provider-azure` (stub)
- `crates/store` SQLite cache
- `crates/engine` facade for UIs
- `apps/cli` the `gv` binary
- `apps/desktop` the Confluo desktop app (GPUI, binary `confluo`)

## CLI

Install with `cargo install --path apps/cli`, then:

```sh
gv project add work --group gitlab:gitlab.com/my-group
gv project add work --folder ~/code --depth 3
gv project add work --repo https://github.com/owner/repo
gv list                      # everything involving you (refreshes first)
gv list --project work --reviewing --needs-you
gv show https://github.com/owner/repo/pull/4
gv comment <url> "body" [--thread <id>]
```

The cache lives in `~/Library/Application Support/Confluo/cache.db` (copied once from the old `git-viewer` dir if present) (override with `GV_DB`).

## Desktop app

```sh
cargo run -p gv-desktop --release   # binary: confluo
```

Built on GPUI (`gpui = 0.2.2` from crates.io, `runtime_shaders` feature so no full Xcode `metal` tool is needed).
The engine runs on a background tokio runtime; results are bridged back to the UI thread.

- Sidebar: Inbox (everything involving you) and projects; `+` adds a folder, repo URL or group (`provider:host/path`) to a project.
- List: Authored / Reviewing / All tabs, search, "Needs you" and "Starred" chips, star toggle per row.
- Detail: status, review, branch, changes, checks, copyable checkout command, description, activity threads.
  Comment, reply (Reply on a thread) and edit your own comments (Edit) from the composer; Enter posts.
- Repo aliases: rename how a repo is shown (sidebar "Repo aliases", or "Alias" in the detail header).
- Appearance: Blurred / Transparent / Opaque window, panel opacity, background image with dim, tint.
  Saved to `~/Library/Application Support/Confluo/settings.json`.
