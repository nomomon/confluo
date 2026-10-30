# Development

```sh
cargo run -p gv-desktop          # desktop app (binary: confluo)
cargo run -p gv-cli -- list      # CLI
cargo test --workspace
scripts/install-macos.sh         # build, bundle and copy to /Applications
```

Dev-only env (screenshots and walkthroughs; see `apps/desktop/src/dev.rs`):

- `CONFLUO_OPEN=<pr url>` previews a PR; `CONFLUO_MODE=review` (or `CONFLUO_TAB=code|activity`) opens it in Review mode; `CONFLUO_SPLIT=1` side-by-side diff; `CONFLUO_SELECT=path:start-end`, `CONFLUO_REVEAL=<thread id>`
- `CONFLUO_SETTINGS=<page>` opens Settings on a page; `CONFLUO_OVERLAY=add-project|rename|delete|menu|palette|shortcuts|review|older` (`CONFLUO_PALETTE_QUERY=<text>` prefills the palette, e.g. `#5` or `>`; shortcuts opens the `?` cheat sheet) (review opens the Review menu of the open PR, older shows the stale section); also `more|merge|close|people|checks` (More menu, merge dialog, close confirm, reviewer picker, first failed check expanded), and `checks-popover|quick-open|repo-menu|repo-rename`
- `CONFLUO_VIEW=starred|bots|project:<name>` picks the list view; `CONFLUO_SCROLL=<px>` scrolls the detail pane
- `CONFLUO_THEME=light|dark|system`, `CONFLUO_MATERIAL=blurred|transparent|opaque` override appearance for the launch
- `CONFLUO_HINTS=1` keeps the hold-Cmd shortcut hints visible
- `CONFLUO_WINDOW=1100x760` window size; `CONFLUO_REDUCE_MOTION=1`; `CONFLUO_NO_REFRESH=1` stays on cached data (no network)

Note: macOS stops repainting the window while the screen is locked, so only the first frames can be captured then;
prefer cached data (`CONFLUO_NO_REFRESH=1`) so the first frame is complete.
Data: `~/Library/Application Support/Confluo/` (override cache with `GV_DB`; avatars in `avatars/`).
`GV_DEBUG_HTTP=1 gv list` prints request counts per host and endpoint after the refresh.

## Crash logs

A panic writes `~/Library/Logs/Confluo/crash-<timestamp>.log` (message, location, backtrace; last 10 kept).
GPUI calls app code from platform callbacks that cannot unwind, so any panic aborts the app; the next launch
shows "Confluo quit unexpectedly last time" with Open log. Never call `ListState` methods (`viewport_bounds`,
`logical_scroll_top`, ...) from inside a `list(..)` item callback: the state is mutably borrowed there.

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

## GitLab request changes

GitLab has no REST endpoint for "Request changes"; it is the GraphQL mutation
`mergeRequestRequestChanges(input: {projectPath, iid})` (on gitlab.com, sets the caller's reviewer state to
`REQUESTED_CHANGES`; `mergeRequestDestroyRequestedChanges` undoes it). The provider probes the instance once with a
read-only `__type(name: "MergeRequestRequestChangesInput")` query and offers the action only when it exists
(older self-managed instances keep it hidden). Reviewer states come from `GET .../reviewers`.

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

## Azure DevOps

Repos are `org/project/repo` on `dev.azure.com` (`*.visualstudio.com` and `ssh.dev.azure.com` remotes are
normalised). Auth: `az account get-access-token` (Bearer, refreshed once on 401) or `AZURE_DEVOPS_EXT_PAT`
(Basic). A failed `az` lookup is remembered for 60s; Accounts and the refresh banner say
"sign in again with `az login`". Add a whole project with `gv project add <name> --group azure:dev.azure.com/org/project`
or by pasting `https://dev.azure.com/org/project`.

- List: `.../_apis/git/pullrequests?searchCriteria.status=active` per project (one request), or per repo;
  the inbox asks each organization of your projects for `creatorId` and `reviewerId` (two requests per org).
  Votes come with the list (10/5 approved, -5 waiting for author, -10 rejected).
- Checks: policy evaluations (build validation, minimum reviewers, comment resolution) plus PR statuses.
  Lists enrich only when the source commit moved or a policy was still running (one request per PR).
- Detail: the PR, threads, commits, policy evaluations and statuses (five requests). Vote events come from
  system `VoteUpdate` threads. Comment ids are `thread/comment`.
- Diff: last iteration's changes, then both blobs per file, diffed locally (`similar`); the first 120 files
  only, blobs over 1 MB are "too large". Cached by source commit like other providers.
- Writes (not exercised against a live org): threads, replies, edits, inline threads (`threadContext`),
  resolve (`fixed` / `active`), votes via `PUT .../reviewers/{me}`, title/description/draft/abandon via
  `PATCH`, complete with merge strategy and branch deletion (or auto-complete), reviewers, comment likes,
  re-queue failed build policies. Viewed files are kept locally.
