# git-viewer

One place for PRs and MRs across GitHub, GitLab and Azure DevOps.

## Layout

- `crates/core` domain types, `Provider` trait, needs-you logic (no IO)
- `crates/auth` tokens from env (`GV_TOKEN_<HOST>`, `GH_TOKEN`, `GITLAB_TOKEN`, `AZURE_DEVOPS_EXT_PAT`), `gh`, `glab`, `az`
- `crates/discovery` find git repos in a folder and classify remotes
- `crates/provider-github`, `crates/provider-gitlab`, `crates/provider-azure` (stub)
- `crates/store` SQLite cache
- `crates/engine` facade for UIs
- `apps/cli` the `gv` binary
- `apps/desktop` the `git-viewer` desktop app (GPUI)

## CLI

```sh
gv project add work --group gitlab:gitlab.com/my-group
gv project add work --folder ~/code --depth 3
gv project add work --repo https://github.com/owner/repo
gv list                      # everything involving you (refreshes first)
gv list --project work --reviewing --needs-you
gv show https://github.com/owner/repo/pull/4
gv comment <url> "body" [--thread <id>]
```

The cache lives in `~/Library/Application Support/git-viewer/cache.db` (override with `GV_DB`).

## Desktop app

```sh
cargo run -p gv-desktop --release   # binary: git-viewer
```

Built on GPUI (`gpui = 0.2.2` from crates.io, `runtime_shaders` feature so no full Xcode `metal` tool is needed).
The engine runs on a background tokio runtime; results are bridged back to the UI thread.

- Sidebar: Inbox (everything involving you) and projects; `+` adds a folder, repo URL or group (`provider:host/path`) to a project.
- List: Authored / Reviewing / All tabs, search, "Needs you" and "Starred" chips, star toggle per row.
- Detail: status, review, branch, changes, checks, copyable checkout command, description, activity threads.
  Comment, reply (Reply on a thread) and edit your own comments (Edit) from the composer; Enter posts.
- Repo aliases: rename how a repo is shown (sidebar "Repo aliases", or "Alias" in the detail header).
- Appearance: Blurred / Transparent / Opaque window, panel opacity, background image with dim, tint.
  Saved to `~/Library/Application Support/git-viewer/settings.json`.
