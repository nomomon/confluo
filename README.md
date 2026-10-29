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
