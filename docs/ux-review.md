# UX review (2026-10-30)

Hands-on walkthrough of Confluo 0.x on macOS, dark and light themes, against `design-system.md`.

## How it was exercised

- No real input: the terminal had no Accessibility permission (osascript and cliclick events were
  rejected) and the screen was locked during the session. Every state was reached through dev env
  hooks (`docs/development.md`) and captured per window with `screencapture -l`.
- macOS stops repainting a window while the screen is locked, so only the first frames are visible.
  Async states (spinners settling, avatars arriving, refresh results) could not be watched.
- Not exercised: hover, drag, typing, keyboard shortcuts (j/k, Esc layering, /, Cmd+R, Cmd+,, n/p, x,
  arrows), motion timing, the Blurred material over a white or dark window (needs the unlocked desktop;
  the readability floors are covered by `translucency.rs` tests against pure white).

## Findings

Status: fixed, open.

### P0

1. **Add project sheet renders as a small box in the top-left corner** over the traffic lights instead
   of a full-height panel on the right. `motion::slide_in_right` forced `relative()`, overriding the
   sheet's absolute position. Fixed: `motion::slide_in_from_edge` keeps the position. (fixed)
2. **Accounts never finishes loading**: the page waited for a live "who am I" per host, which queues
   behind a running refresh in the per-host governor (and never resolves during a rate limit pause).
   Fixed: rows appear at once from the cached usernames, the network check is time boxed (8s) and keeps
   the cached user on timeout, and a dropped task no longer leaves the skeleton up. (fixed)

### P1

3. **List too narrow with the detail pane open**: at 1280px titles cut at about 25 characters and the
   meta line was clipped mid-word ("7 pa"). Fixed: below 1440px an open detail pane collapses the
   sidebar to an icon rail with tooltips, list padding halves, rows drop author and hint, and the
   detail pane gives way so the list keeps 420px. (fixed)
4. **Opening a PR also waited on "who am I"** before showing fresh detail, so detail stayed on
   "Loading..." while a refresh ran. Fixed: uses the cached username. (fixed)
5. **Code tab showed "Loading files..." on every relaunch** although the diff was in the disk cache.
   Fixed: the Code tab paints from the disk cache and revalidates quietly. (fixed)
6. **Starred view was titled "Inbox"** and its empty state said "No open pull requests" with an
   "Add project" button. Fixed: "Starred" title and a starred-specific empty state. (fixed)
7. **Filters: the hidden authors input collapsed to a 26px sliver** once the chips wrapped. Fixed:
   shadcn style tag input, the field follows the chips and takes the rest of the line. (fixed)
8. **Dialogs and sheets were see-through**: `overlay()` at 97% let list text show through Settings.
   Fixed: opaque. (fixed)

### P2

9. **Code tab file header hid the file name** (path truncated from the right). Fixed: file name first,
   directory muted after it and truncating. (fixed)
10. **Markdown table headers wrapped mid-word** ("Confidenc e"). Fixed: header cells get room for
    semibold text. (fixed)
11. **No ellipsis on truncated text**: GPUI 0.2.2 measures nowrap text once before flex gives it a
    width, so `truncate()` clips without "...". Framework limit; revisit on a GPUI upgrade. (open)
12. **"Loading..." in the detail meta line** shows even when cached detail is already on screen;
    reads as busy. A quieter freshness hint would fit better. (open)
13. **Tint stored as a custom hex** (`#0b1220`) matches no preset, so no swatch looks selected.
    Consider snapping to the nearest preset or showing a "Custom" swatch. (open)
14. **Narrow window (900px) with detail open**: even with the rail the list is tight (repo label cut to
    3 letters). Consider auto-hiding the repo label below a list width, or opening detail as an overlay
    under 1000px. (open)
15. **Showing bots floods "Needs you"** (14 items, mostly Renovate). Consider keeping shown bot PRs in
    their own section. (open)
16. **Consistency**: Settings nav, sidebar and menus share `NavItem`; dialogs, sheets and menus use the
    kit. No ad hoc colors were seen in the captured screens. Avatars show initials in every captured
    frame (images arrive after the first frame), so avatar loading could not be judged. (open, unverified)

## Screenshots

Before: `b-*` and `d-*`; after: `c-*`, `e-*`, `f-*`, `g-*` in the session scratchpad `walk/` folder.
