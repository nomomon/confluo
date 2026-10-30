# Desktop design system

The GPUI desktop app (`apps/desktop`) is built from a small in-house UI kit.
Visual target: minimal dark, rounded cards with very subtle borders, muted
secondary text, and color only where it carries meaning.

## Layering

```
app.rs      root composition only (window, background, regions, overlays)
features/   organisms and views: sidebar, pr_list, pr_detail, dialogs
state/      app state transitions and the tokio-to-GPUI async bridge
ui/         app-agnostic kit: theme, motion, atoms, molecules
```

Rules:

- `ui/` must never import `crate::features`, `crate::state` or `gv_engine`.
- `features/` composes `ui/` components and reads/writes app state.
- `state/` contains no rendering.
- Stateless primitives are `RenderOnce` structs with builder APIs, for example
  `Button::new(id, label).variant(ButtonVariant::Ghost).size(Size::Sm).icon("+").on_click(..)`.
- Features never use raw colors, pixel values or durations. Use tokens from
  `ui::theme` and `ui::motion`; fixed region sizes live in
  `features/shared/layout.rs`.

## Color

`ui::theme::palette` holds raw scales; features use only semantic tokens.
Tokens are functions (`theme::text_muted()`), resolved against the live
palette, so the theme switches without a restart.

Neutral scale `palette::zinc::Z50 .. Z950` (Tailwind zinc). Accents live in
`palette::dark` (light 400-ish shades) and `palette::light` (darker shades for
contrast on grey), used only where meaning exists:

| Accent | Meaning                     | Semantic token |
|--------|-----------------------------|----------------|
| green  | open, added, passed         | `success()`    |
| purple | merged                      | `merged()`     |
| red    | removed, failed, closed     | `danger()`     |
| amber  | pending, needs you, starred | `warning()`    |
| blue   | links, focus                | `accent()`, `focus_ring()` |

Semantic tokens (`ui::theme::*`), dark / light:

| Token              | Dark                  | Light                       |
|--------------------|-----------------------|-----------------------------|
| `bg()`             | zinc 950              | page grey ~#f3f3f5          |
| `surface()`        | white 4%              | black 4%                    |
| `surface_hover()`  | white 7%              | black 6%                    |
| `surface_active()` | white 10%             | black 9%                    |
| `border()`         | white 8%              | black 9%                    |
| `border_subtle()`  | white 5% (dividers)   | black 6%                    |
| `text()`           | zinc 100              | zinc 900                    |
| `text_muted()`     | zinc 400              | zinc 600                    |
| `text_subtle()`    | zinc 500              | zinc 500                    |
| `overlay()`        | zinc 900, opaque      | zinc 50, opaque             |
| `scrim()`          | black 45%             | black 25%                   |

Code tokens: `diff_add_bg()`, `diff_del_bg()`, `diff_hunk_bg()`, `line_selected_bg()` (accent 16% dark /
14% light, layered over the add/del fill of selected lines) and `syntax_keyword()`,
`syntax_string()`, `syntax_number()`, `syntax_type()`, `syntax_comment()` (hues in
`palette::syntax`, one shade per theme).

`tint_bg(color)` gives the 14% (12% light) fill used behind pills, badges and
notices.

### Theme

Settings > Appearance > Theme: System / Light / Dark (`settings::ThemeMode`, default
System). System follows `window.appearance()`; the root view observes
`observe_window_appearance` and re-renders, calling `theme::set_light(..)`
each frame before children render.

### Translucency

`Translucency::new(panel_opacity, tint, base_alpha)` derives fills from the
appearance settings (`GitViewer::surfaces()` builds it). The tint is one of the
saturated presets in `settings::TINTS` (None, Slate, Blue, Violet, Rose, Amber,
Emerald, Teal), mixed in sRGB (`theme::mix`) into the neutral fills: 16% into the
window background, 7% into panels and cards (60% of that in light mode).

- `window_bg()`: zinc 950 (light: page grey) plus tint, alpha
  `max(base_alpha, floor)`; base is 1.0 opaque, 0.35 blurred, 0.55 transparent
- `panel()`: zinc 900 (light: `light::PANEL`) plus tint; panel opacity maps
  0..1 onto `floor..1`
- `card()`: zinc 800 (light: `light::CARD`) at 35-70% alpha

Readability floors guarantee text contrast >= 4.5:1 whatever is behind the
window (tested in `translucency.rs` against pure white): dark window 0.9, dark
panels 0.82, light window 0.86, light panels 0.8.

## Spacing, radius, type

Spacing, Tailwind 4px steps: `SPACE_0` 0, `SPACE_0P5` 2, `SPACE_1` 4,
`SPACE_1P5` 6, `SPACE_2` 8, `SPACE_2P5` 10, `SPACE_3` 12, `SPACE_4` 16,
`SPACE_5` 20, `SPACE_6` 24, `SPACE_7` 28, `SPACE_8` 32, `SPACE_10` 40,
`SPACE_12` 48, `SPACE_14` 56, `SPACE_16` 64.

Rhythm tokens are functions driven by Settings > Appearance > Density (Compact default /
Comfortable), so features never hard-code breathing room:

| Token                  | Compact | Comfortable | Use                                  |
|------------------------|---------|-------------|--------------------------------------|
| `control_h()`          | 32      | 36          | tabs, search, inputs                 |
| `row_min_h()`          | 52      | 62          | two-line list rows (render ~56-60)   |
| `row_pad_y()`/`row_pad_x()` | 8 / 12 | 12 / 16  | list row padding                     |
| `row_line_gap()`       | 4       | 6           | title to meta line                   |
| `kv_row_h()`           | 36      | 44          | key/value rows                       |
| `page_pad_x()`         | 32      | 48          | list column sides (`_narrow()` 20/24 with the detail open) |
| `section_gap()`        | 20      | 28          | above section headers, timeline items |
| `section_header_gap()` | 8       | 12          | header to card                       |
| `card_gap()`/`card_pad()` | 12 / 14 | 16 / 16  | stacked cards, card padding          |
| `pane_pad()`           | 20      | 24          | detail pane sides                    |
| `item_pad_y()`         | 5       | 7           | nav, menu, palette, select items     |

Targets come from Zeron measured in points (rows ~68pt with 15px titles there; we go tighter,
Linear/Mail-like). `KV_KEY_W` 88. Layout: `CONTENT_MAX_W` 1360, `SIDEBAR_W` 240,
`TITLEBAR_INSET` 48.
Scroll past end: the review diff list ends with a spacer item (not a row) of
`SCROLL_PAST_END` 40% of its viewport (320 before it is measured); the PR list,
timeline and file tree end with `END_PAD` 96.

### List items

Every selectable row uses the `ListItem` trait (`ui/atoms/list_item.rs`):
`div().id(..).list_item(selected)` gives a rounded (`LIST_ITEM_RADIUS` 8) selected fill
(`surface_active()`), hover (`surface_hover()`) and press. Used by `NavItem` (sidebar, settings nav),
`MenuItem`, the command palette, `Select` options, the Code tab file tree and PR rows. Rows in a
section card sit inside a `LIST_INSET` (4) padded card of radius `LIST_RADIUS` (12 = 8 + 4), so the
highlight is an inset rounded rect for every row, never a square edge.

### Spacing rhythm

Calm but compact. The list column is centered, capped at `CONTENT_MAX_W`, with
`page_pad_x()` side padding and 48px top padding.

- Page title `text_2xl()` (22) semibold, subtle count `text_base()`, then `card_gap()`
  to the controls row (`control_h()` tall).
- Section header (12px chevron, label `text_sm()` medium muted, count): `section_gap()` above,
  `section_header_gap()` below, outside the card.
- One rounded card (`LIST_RADIUS`) per section, padded by `LIST_INSET`; rows are `ListItem`s split
  by 1px `border_subtle()` dividers inset to the row text (`ListSection`).
- Rows: `row_min_h()`, `row_pad_y()`/`row_pad_x()`; title `text_md()` (13.5) medium; meta
  `text_xs()` muted with `SPACE_2` between items; right column time over `+a -d` in `text_xs()`;
  star outside it.
- Meta color: CI passed `success()`, failing `danger()`; hints such as "Your
  review requested" and "Draft" are muted; the number is a green tinted
  `Pill` with the git-pull-request icon.
- Sidebar: `SPACE_3` padding, nav items `SPACE_2` vertical.
- Narrow windows: with the detail pane open below `COMPACT_BELOW` (1440) the sidebar collapses to an
  icon rail (`SIDEBAR_RAIL_W`, `NavItem::compact`, labels become tooltips), the list padding drops to
  `SPACE_6`, rows drop the author and hint, and the detail pane narrows so the list keeps `LIST_MIN_W`. Detail pane:
  `pane_pad()` sides, cards `card_gap()` apart, title `text_xl()` (18) semibold, meta `text_xs()`,
  key/value rows in `Card::rows()`.

Code tab: files, hunks and lines flatten into one `Row` list (`state::code_rows`) rendered
with GPUI `list` so only visible rows paint; each file reads as a rounded card made of
its rows. Sizes `DIFF_LINE_H`, `DIFF_HEADER_H`, `DIFF_HUNK_H`, `DIFF_GUTTER_W`, `FILE_TREE_W`
live in `features/shared/layout.rs`. Shortcuts n/p or ]/[ and x act only on the Code tab.

Line selection (Code tab and "View file"): `state::line_select` holds the model, an anchor and a
head over one hunk of one file (`DiffSel`) or the viewer's lines (`LineRange`); extending past the
hunk clamps to its edge, so a selection never spans files. It is independent of the comment
composer. Mouse: click a line (gutter or code) selects it, Shift+click extends, drag selects a range
and scrolls the list near its edges; a click on empty space or Esc clears. Keyboard (Code tab only):
up/down move a one-line cursor across hunks and files and keep it in view, Shift+up/down extends,
`c` or Enter opens the composer on the range, Cmd+C copies; j/k still switch PRs and n/p files.
Selected lines get `line_selected_bg()` and a 2px `accent()` bar in the gutter (`DiffRow::selected`).
The hover "+" comments on one line, or drag from it across lines and release to open the composer on the range
(GitHub style; header "Comment on lines 12-18"). A `FloatingToolbar` hangs under the last selected line
(Comment, Copy, Copy link, Open in browser); links are GitHub `{pr}/files#diff-{sha256(path)}R3-R7`
in the diff, else blob permalinks at the head sha (`state::permalink`). Character-level text
selection inside a line is not implemented. Dev: `CONFLUO_SELECT=path:start-end` preselects lines.

Icons: `ICON_SM` 12 (chevrons, section headers), `ICON_MD` 14 (buttons, nav), `ICON_LG` 16 (row state).
Never use text glyphs as icons; add a Tabler outline SVG (stroke-width 1.75) and an `IconName` variant instead.

Radius: `RADIUS_SM` 6, `RADIUS_MD` 8, `RADIUS_LG` 12, `RADIUS_XL` 16,
`RADIUS_FULL`.

Type (functions, so the user's text size applies live): `text_2xs()` 10,
`text_xs()` 12, `text_sm()` 13 (body default), `text_base()` 14, `text_lg()` 16,
`text_md()` 13.5 (row titles), `text_lg()` 16, `text_xl()` 18 (detail title), `text_2xl()` 22
(page title). Settings > Text size (13/14/15) adds 0/1/2 px to every step. The root sets line
height `LINE_HEIGHT` 1.4 (GPUI's default is 1.618, which made everything airy). Weights: 400 body,
500 (`WEIGHT_MEDIUM`) for row titles, names, card titles and section labels; 600 (`WEIGHT_SEMIBOLD`)
only for page, detail, dialog and settings titles and markdown bold; `WEIGHT_BOLD` is unused.

### Fonts

Bundled in `apps/desktop/assets/fonts` (SIL OFL 1.1, license next to each
family), weights 400/500/600/700, embedded and registered by `fonts::register`
via `cx.text_system().add_fonts` at startup:

| Family          | Source                         | Role             |
|-----------------|--------------------------------|------------------|
| Inter           | rsms/inter v4.1                | UI               |
| Geist           | vercel/geist-font v1.8.2       | UI               |
| JetBrains Mono  | JetBrains/JetBrainsMono v2.304 | mono (default)   |
| Geist Mono      | vercel/geist-font v1.8.2       | mono             |

Settings offers UI font System (`.SystemUIFont`, SF Pro, default) / Inter / Geist and
mono font Menlo / JetBrains Mono / Geist Mono. System is the default: compared side by side
at the same size, Inter's static text cuts render a step heavier in GPUI (medium reads as
semibold), while SF Pro gets the native optical sizing and tracking and matches Mail and Finder. Read the live families with
`theme::ui_font()` and `theme::mono_font()` (branches, commands, SHAs). The app
pushes `Settings::typography()` into the theme each frame, so changes apply
without a restart.

## Components

shadcn/ui names and behavior where GPUI allows. All are `RenderOnce` with builders; value
callbacks take `&T` (`ui::ChangeHandler<T>`). Components needing internal state (open
select, slider bounds, hovered link) keep it with `window.use_keyed_state`.

Atoms:

- `Avatar`: `Avatar::new("ada").image(cache.get(url))`, initial while loading
- `Badge`: `Badge::new("Draft").variant(BadgeVariant::Outline)`, or `.tint(theme::success())`
- `Button`: `Button::new(id, "Save").variant(ButtonVariant::Primary)` (Outline, Ghost, Link, Primary, Destructive), `.icon(..)`, `.tooltip("Refresh  ⌘R")`
- `Checkbox`: `Checkbox::new(id, done).label("Ship").on_change(|v, w, cx| ..)`
- `Chip`, `Pill`, `NavItem` (`.icon(..)`, `.trailing_on_hover(..)`, `.on_right_click(..)`)
- `Icon`: `Icon::new(IconName::BrandGithub).size(ICON_MD)`; Tabler outline SVGs in `assets/icons` (MIT)
- `Kbd`: `Kbd::new("⌘ R")`, one box per space separated key
- `Separator`: `Separator::new()` or `Separator::vertical()`
- `Spinner`: `Spinner::new(id).color(theme::warning())`, rotating loader, still under reduced motion
- `Skeleton`: `Skeleton::new().w(px(120.)).h(px(12.))`, pulses unless reduced motion
- `Swatch`: `Swatch::new(id, color).active(true)`, selected ring
- `Switch`: `Switch::new("hide-bots", on).on_change(|on, w, cx| ..)`
- `tooltip(text)`: for any stateful element; text after a double space renders as `Kbd`

Molecules:

- `Banner`: one-line dismissible error with an action such as Retry
- `CommandPalette`: `CommandPalette::new(input).items(Vec<PaletteItem>).selected(i).footer(hint).on_move(..).on_hover(..).on_run(..).on_alt_run(..).on_close(..)`;
  Up/Down (Ctrl+P/N) move, hover highlights, Enter runs, Cmd+Enter alt-runs, no open animation; `command_palette::score` is the fuzzy matcher.
  The app palette is the command hub: every `keymap::ENTRIES` row is a command (its `available(&CmdCtx)` predicate decides
  when it shows, its keys render as `Kbd`); ranking, prefixes (`>` `#` `@` `/` `?`) and the MRU live in `state::palette_rank`
- `Combobox`: `Combobox::new(id, input).option(ComboOption::new("ada").detail(name).avatar(img).selected(true)).highlight(i).loading(b).on_toggle(..).on_highlight(..).on_dismiss(..)`;
  searchable multi-select popover, the caller owns the query input and (remote) filtering
- `Card`: `Card::new(s).title("Checkout").description("..").action(btn).footer(..)`; `.rows()` for flush divided rows
- `ContextMenu`: `ContextMenu::new(id, event.position).on_dismiss(..).child(MenuItem::new(..))`
- `CopyField`, `EmptyState`, `KeyValueRow`, `FloatingToolbar` (selection pill; `.animate(false)` for keyboard moves)
- `Dialog`: `Dialog::new("Delete").description("..").content(..).footer(buttons)`; `Dialog::bare().size(w, h)` for custom chrome
- `MenuItem::disabled(true).tooltip("Why")` for an action that exists but is not allowed (e.g. approving your own PR)
- `Popover`: `Popover::new(id).placement(Placement::BottomEnd).offset(gap).on_dismiss(..).child(panel)`, the one primitive for
  floating panels: rendered inside a `relative()` wrapper of the trigger, painted through `deferred(anchored())` at the window
  root (escapes clipping and later siblings such as the right panel), flips then shifts to stay on screen, and records its
  bounds in `ui::layer` so tooltips and hover cards outside it stay hidden. `ContextMenu`, `Select` and `Combobox` use the same
  `deferred(anchored())` and `layer::track`.
- Floating layers: `state::Layer` (More, Review, project menus, checks popover, reaction picker, row context menu) is the one open
  transient layer; `open_layer` / `toggle_layer` close the others (and the repo menu and people picker), a press outside calls
  `dismiss_layer` (the same press on the trigger does not reopen it), Esc closes it, hold-Cmd hints hide while one is open.
- `DropdownMenu`: `DropdownMenu::new(id).placement(..).on_dismiss(..)` (a `Popover`, default `BottomEnd`) with `MenuItem::new(id, "Delete").icon(..).shortcut("⌘ D").destructive()`, `MenuSeparator`, `MenuLabel::new("Project")`
- `Field`: `Field::new("Source", input)`; settings row `Field::new("Refresh", select).description("..").horizontal()`
- `Input`: `Input::new("search", entity, has_text).leading(IconName::Search).on_clear(..)` (create the entity with `TextInput::new(..).inset(INPUT_INSET, INPUT_INSET)`)
- `ListSection`: collapsible header and count above a rounded card of divided rows
- `RadioGroup`: `RadioGroup::new("launch").option("Inbox").option("Last view").selected(0).on_change(..)`
- `Select`: `Select::new("font").font_option("Geist", "Geist").selected(i).on_change(..)`; Up/Down/Enter/Esc on the focused trigger
- `Sheet`: `Sheet::new("Add project").description("..").on_close(|w, cx| ..).child(..)`, slides from the right (`PANEL`)
- `Slider`: `Slider::new("opacity", v).range(0., 1.).step(0.05).on_change(..)`, click or drag
- `Tabs`: `Tabs::new(id).variant(TabsVariant::Underline).tab("Code", true).on_select(..)` (Pill, Underline, Floating)
- `Textarea`: `Textarea::new(composer).rows(3).max_rows(12)` around a multiline `TextInput`: grows from `rows` to `max_rows`,
  then scrolls inside (wheel, caret kept in view); the input draws the single border and the focus ring.
  `TextInput::markdown()` (comment composers, description): Enter continues lists (`-`, `*`, `1.` counting up,
  `- [ ]`) and ends them on an empty item, Tab / Shift+Tab indent and outdent list items, Cmd+B / Cmd+I / Cmd+K
  wrap the selection in `**`, `_`, `[](url)` (bound in the `markdown` key context, so the app's Cmd+B / Cmd+K
  do not fire while it has focus), and light highlighting (`md_highlight`: headings bold, markers muted, code,
  bold, italic, links accent; one font size). `.document()` drops border, fill and inset for in-place editing
- `Toaster`: `Toaster::new().toast(Toast::new("Copied").description("..").variant(ToastVariant::Success))`, bottom-right stack
- `ToggleGroup`: `ToggleGroup::new("theme").item("Dark", None, true).on_change(..)`
- `code::{CodeText, DiffRow, Lang}`: mono line with a line-based tokenizer; `DiffRow` adds gutters and tints
- `markdown::Markdown`: `Markdown::new(id, src).images(loaded)`: rhythm, h1-h6, nested lists, task checkboxes,
  code fences with syntax colors and copy, alerts as tinted callouts, tables (header, borders, alignment,
  horizontal scroll: `table::hscroll` is a flex row so its child keeps max-content width, with a thin thumb when it overflows), collapsible `<details>`, hover-underlined links, autolinks, footnotes; remote images
  come from the avatar cache (`markdown::image_urls` lists them), else alt text
- `TextInput`: `.multiline()`: Enter inserts a newline, Cmd+Enter submits; `.tabs()` makes Tab insert two spaces.
  macOS editing: arrows (Up/Down keep the goal column across wrapped rows), Option word moves, Cmd line and document
  ends, Shift extends each; Option/Cmd+Backspace, double/triple click, drag, Cmd+Z / Shift+Cmd+Z. Caret logic is the
  pure `text_input::model` (rows of caret stops, word ranges, undo history), unit-tested

Settings (`features/settings/`) is a large `Dialog::bare()` (780x560): left nav of pages
(General, Appearance, Accounts, Projects, Filters, Shortcuts), right side made of
`settings_group` cards of horizontal `Field` rows. Cmd+, toggles it. Dev: `CONFLUO_SETTINGS=appearance`
opens a page at launch; `CONFLUO_REDUCE_MOTION=1` forces reduced motion (screenshots).

Shortcuts: `features/keymap.rs` is the only place bindings are defined (keys, context, label, cheat sheet
group, palette placement, icon and an `available(&CmdCtx)` predicate). Menus (`features/menus.rs`) show them
as key equivalents, tooltips use `keymap::tip::<Action>("Label")`, menu rows `keymap::kbd::<Action>()`, and
the palette, the `?` cheat sheet and Settings > Shortcuts are generated from it. Bare character keys never fire
inside text inputs. PR actions (approve, request changes, merge, draft, close) are gated by capabilities
(`state::cmd_ctx`): not allowed means a neutral toast with the reason, never a silent no-op.

Shortcut hints: holding Cmd alone for `timing::HINT_HOLD` (250ms) shows a `ShortcutHint` (inverted pill, `text_hint()` 11,
`BLINK` in (60ms linear opacity, no easing or scale), hidden instantly, deferred so it paints on top) over controls that have a Cmd shortcut; releasing Cmd, any key press or the window
losing focus hides them, so Cmd+key never flashes them. `state::hint_hold` is the state machine; `features/hints.rs` tracks
modifiers on the root and gives `self.hinted::<Action>(el, HintAnchor::Corner | Inline | Below)` and `hint_group(..)`, which read
keys and `available(&CmdCtx)` from the keymap (unavailable actions show nothing). Settings > General can turn it off.

Activity tab: a timeline, not cards. 24px avatar, name medium, muted relative time (exact time tooltip),
markdown body, no per-comment border; a 1px `border_subtle()` rail through the avatar centers; replies
indent under the parent with 20px avatars. Comment actions have one registry (`pr_detail::comment_actions`:
label, icon, menu group, availability from ownership, thread state and provider): hovering a comment shows a floating
toolbar at its top right (overlapping the top edge, no layout shift; hidden while another layer is open) with the three
most recently used reactions (`reaction_mru` in settings, default thumbs up, heart, tada), add reaction (the picker),
Reply, Resolve / Unresolve and More; More and right-click open the same menu (`comment_menu`): an emoji row, then
Reply / Quote reply, Resolve, Copy text / Copy link to comment (GitHub `#issuecomment-{id}` / `#discussion_r{id}` from the
node id, GitLab `#note_{id}`, Azure `?discussionId=`) / Open in browser / Show in code, Edit / Delete last. Delete asks
"Delete comment? This can't be undone.", removes at once and restores with a toast if the provider refuses; an inline
thread's last comment takes the thread (`Provider::delete_comment`, `gv delete-comment`). Right-click is consistent elsewhere, always on the shared `ContextMenu`: PR rows (open,
open review, open in browser, copy link, star, rename repository, snooze, hide), file tree items (mark viewed, view
file, copy path, open in browser), diff lines (comment, copy lines, copy link, view file; a line outside the selection
is selected first), repo names (rename, hide). GitLab notes fetch their
award emoji lazily (`state::awards`: notes on screen queue, a debounced batch of 25 through the governor, cached by note
id and re-applied on reload); Azure has likes only. Events (opened,
approved, changes requested, merged, closed) sit on the rail as small icon circles with plain glyphs (check, x), never a
circled icon inside the circle. The composer is pinned under the tab pill.

Stale PRs (no update for `stale_days`, default 60) and snoozed PRs leave the sections; one quiet footer line
says what is hidden ("12 bot PRs and 8 older PRs hidden. Show bots · Show older").

List sections (Needs you, Awaiting review, Yours, Drafts, Waiting on others) apply in every
view; collapse state is saved per view (`inbox`, `starred`, `project:<id>`) in settings.json.

Show on hover with `.group(..)` plus `.group_hover(..)`; GPUI has no visibility
toggle, so hidden content uses `opacity(0.)`.

Timers that are not animations (auto refresh, loading delay, "Copied" reset,
toasts) live in `state::timing`.

## Motion

Tokens in `ui::motion` (`Motion { ms, easing }`), all capped at `MAX_MS` = 300:

| Token         | Duration | Easing         | Use                                  |
|---------------|----------|----------------|--------------------------------------|
| `INSTANT`     | 0        | linear         | explicit no-animation                |
| `HOVER`       | 110ms    | ease-out quint | hover feel (style swap, see below)   |
| `PRESS`       | 70ms     | ease-out quint | pressed feel (style swap)            |
| `FADE`        | 150ms    | ease-in-out    | notices, scrims, toasts              |
| `BLINK`       | 60ms     | linear         | shortcut hints (feel instant)        |
| `PANEL`       | 220ms    | ease-out quint | panel entrances                      |
| `PANEL_EXIT`  | 154ms    | ease-out quint | exits, ~70% of entry                 |
| `LIST_INSERT` | 180ms    | ease-out quint | newly inserted list items            |

Helpers wrap GPUI `with_animation` with `Animation::new(d).with_easing(..)`
(easings come from `gpui::{linear, ease_in_out, ease_out_quint}`):

- `fade_in(el, id)`: opacity 0 to 1 over `FADE`
- `slide_in_right(el, id, distance)`: offset from the right plus fade, `PANEL` (elements in the flow; it makes them `relative`)
- `slide_in_from_edge(el, id, distance)`: same for an `absolute().right_0()` element such as `Sheet`, keeping its position
- `pop_in(el, id)`: 6px rise plus fade, `FADE` timing with quint ease
- `animate(el, id, motion, animator)`: the generic building block
- `Motion::duration()` / `Motion::animation()` for custom cases

Where it is applied: detail pane slides in on open; dialogs fade the scrim and
pop the panel; dropdown menus pop; the line selection toolbar pops (not on keyboard moves); status notices fade in. Buttons, chips, nav
items and list rows use `.hover()` with `SURFACE_HOVER` and `.active()` with
`SURFACE_ACTIVE`.

Rules:

- Do not animate keyboard-driven repeated actions (j/k, tab switches, command
  palette), data refreshes, or text changes.
- Animate entrances only. GPUI removes elements immediately, so exits are not
  animated yet; `PANEL_EXIT` is the token to use once deferred removal exists.
- GPUI has no style transitions; hover and press are instant style swaps.
  `HOVER` and `PRESS` record the intended timing for when that changes.
- Animation ids are stable per element, so an entrance plays once per mount
  (switching the selected PR does not replay the detail slide).
- Reduced motion: `motion::init()` reads
  `defaults read com.apple.universalaccess reduceMotion` once at startup. When
  on, every duration is 0 and helpers render the final frame without wrapping
  the element in an animation (a zero-length GPUI animation would divide by
  zero).

Editing (Summary): click the title to edit (Enter saves, Esc cancels); the description card shows Edit on
hover; the editor keeps the rendered column, font size and line height (`MARKDOWN_LINE_HEIGHT`) and at least the
rendered height (`DESC_EDITOR_MIN_H` minimum), with Write / Preview, Cancel and Save in a footer (Cmd+Enter saves). A properties card lists Reviewers, Assignees and Labels; editable
rows open a `Combobox` of candidate users (GitHub `assignableUsers`, GitLab project members, 250ms debounce).
The More menu holds Mark as draft / Ready for review, Edit title, Copy branch name, Close / Reopen (Close
confirms) and a disabled Merge with the blocking reason; Merge sits beside Review only when the provider
says it is mergeable now. Everything is driven by `EditCapabilities`, applied optimistically and rolled back
with an error toast.

Checks card: one plain-language line (`gv_core::checks_sentence`), failing/running/pending rows first, passed
and skipped behind "Show N passed"; rows show status (spinner when running), name, stage or workflow,
duration, finish time and an open-logs button; failed rows expand to a lazily fetched log tail. "Re-run
failed" appears when `ChecksInfo::can_rerun` and asks first.

### Motion: physics

Direct manipulation is physical, not tweened. `ui::motion::physics` is pure and app-agnostic:

- `Spring` (`SpringParams::new(response, damping_fraction)`, SwiftUI-style, converted to stiffness / damping / mass;
  semi-implicit Euler in 1/240 s sub-steps, frame gaps capped at 100ms, NaN-safe; starts from any velocity).
  Presets: `SNAPPY` (response 0.3 s, critically damped: pages, rubber band), `BOUNCY` (0.35 s, 0.85: slight
  overshoot), `GENTLE` (0.5 s, critically damped: large panels).
- `VelocityTracker`: release velocity from the last ~100ms of samples, newer ones weighted more; fingers that
  rested before lifting give 0.
- `project(position, velocity, 0.998)`: Apple's momentum projection (`p + v_ms * d / (1 - d)`), picks snap targets.
- `rubber_band(overflow, dimension, 0.55)`: Apple's `(1 - 1 / (x c / d + 1)) d`, symmetric; `rubber_band_inverse`
  grabs a moving band without a jump. `Decay` for inertial coasting (same units as `project`).

`ui::motion::interactive` builds reusable controllers on it: `Pager` (N snap points, 1:1 drag with rubber band past
the allowed pages, release projects and springs with the release velocity, interruptible mid-flight, `progress()`
for intermediate rendering, `tick` reports the settled page once), `SwipeGate` (wheel events to a horizontal drag:
trackpad phases only, shared axis lock, no momentum), `ElasticScroll` (overscroll) and `Animated` (an `f32` springing
to its target for spatial, non-gesture transitions). Frames are requested only while something moves. Reduced
motion: pagers snap, elastic scrolls hard-stop; plain opacity fades stay on the tokens above.

## Triage and Review mode

Triage: the list takes the full width; the selected row shows a slim preview (`PREVIEW_W` 360, Cmd+\\ hides it;
clicking the previewed row again hides it too, after `timing::DOUBLE_CLICK` so a double click never flickers it, and clears
the row highlight: selected means previewed. j/k move the cursor and preview; Esc clears both. Cmd+\\ keeps the keyboard cursor highlighted).
The preview shows the description start as rendered markdown (about six lines, `PREVIEW_EXCERPT_H`) fading into the panel.
Enter, a double click or "Open review" enters Review mode: the PR takes the whole content area (top bar of `REVIEW_BAR_H`:
one line: back, state icon, the short read-only title with ellipsis (repo #n and branch on hover), the unresolved
count, then Review, Merge, More; +/- counts sit in the file tree header beside the viewed progress; file tree,
diff, right panel with Overview / Conversation). The Overview starts with the large inline-editable title (the only place
it is edited), then a compact properties block: Repository (repo #n, right-click renames), Reviewers (with review state), Assignees, Pipeline (CI icon and summary,
click opens the checks popover), Approvals (one "1/1 approved" indicator that reconciles the provider's approval rule
with reviewer states, approvers on hover), Labels, Branch (click copies); then the description. The file tree keeps its own width (resizable, collapsible); the diff and
the right panel share the rest proportionally, 2/3 and 1/3 by default (`layout::right_pane_w`, panel min `RIGHT_MIN_W`,
diff min `DIFF_MIN_W`), so they adapt to the window; dragging stores the tree width and the panel share in settings. Esc, Cmd+[, Cmd+Left or the back chevron return to the list with selection
and scroll kept; Cmd+[ / Cmd+] (and Cmd+Left / Cmd+Right outside text inputs, where they move the caret) walk the visited places (the list and reviewed PRs). Files use one order everywhere (`file_tree::tree_cmp`: folders before files at each level, case-insensitive, so root
files come last): the tree, the diff list, n/p and "File i of N". File tree: viewed checkboxes
(GitHub `markFileAsViewed`, local fingerprint elsewhere), filter, Hide viewed, foldable folders; n/p, v, Cmd+P,
Cmd+Shift+E, Cmd+Shift+I, Cmd+Shift+S (unified / side by side, saved as `diff_split`). CI is one icon
(`shared::ci`): hover lists jobs, click opens the checks popover, nothing when there are no checks.
Reactions: chips plus a picker of eight under comments and the description (`pr_detail/reactions.rs`).
Repository names: right-click to rename or hide; Settings > Repositories lists them by host.

Thread attention (`gv_core::thread_attention`, `state::attention`): each thread is Needs me (unresolved and I am
mentioned, or someone else spoke last on a thread of a PR I author or review, or one I wrote in), Open (unresolved,
waiting on others), Outdated, Resolved, or Plain (not resolvable). GitHub `isResolved`, GitLab `resolvable`/`resolved`
and Azure thread status all map to `Thread::resolved`. Unresolved threads get a 2px left bar (`warning()` for Needs me,
`accent()` for Open) and "Needs you" / "Unresolved" pills, in the Conversation and on diff thread cards; resolved threads
collapse to one muted line with a check (click to expand); outdated ones are muted with "Outdated". The Conversation has
an All / Unresolved / Needs me toggle with counts and "Next unresolved" (`u`, Shift+U back; needs-me first, in the diff
when the thread sits on a current line, and scrolled into view in the Conversation). The file tree shows an amber dot for
files with threads that need me; the review top bar shows "3 unresolved" (amber when some need me), click jumps.

Scrolling: panes scroll y only through `ui::scroll::y_only` (overflow y plus `restrict_scroll_to_axis`, so a sideways swipe
never becomes vertical scrolling). Horizontal blocks (`markdown::table::hscroll`, diff rows) lock each trackpad gesture to
its gesture's axis. Gesture ownership (`ui::arbiter`): one trackpad gesture (fingers down until the next fingers down,
so its momentum too) has one axis, decided by its first movement, and at most one owner, decided by where it starts:
the innermost handler that wants it claims it first. A block takes a sideways gesture only if it could still scroll
that way as it began (then it keeps the x, rubber-bands at its edge and stops the event); a gesture starting in the
right panel belongs to the panel pager (rubber band at the ends, never chaining into the page swipe); else, in Review
mode, the page pager. Once a pager or block owns a gesture every event of it is theirs: vertical scrollers (also the
list revealed under a swipe) put back the wobble GPUI applied (`elastic::guard_vertical`, to the offset they were drawn
at); a vertical gesture is claimed for panes and never reaches a pager. Mouse wheels have no owner.

Swipe between pages (Settings > General, on by default; a `Pager` in `state::swipe_nav`): in Review mode a
two-finger horizontal trackpad swipe that no block claimed (see above) moves the review with the fingers (1:1, rubber band
where there is nothing to go to, and past a full page) over the list view, which sits dimmed at a -30% parallax offset (every layer keeps the full window size and only
translates, over an opaque floor);
swiping left goes forward when history has a PR ahead (it comes in from the right); with none ahead the review stays put. Release projects the position with the
release velocity and completes when that lands past a third of the width (`Pager::commit` 0.33), so a slow drag past a
third or a short flick completes, else it springs back (the spring starts at the fingers' velocity); then it walks history like
Cmd+[ / Cmd+]. Mouse wheels, momentum and vertical gestures never start it; the slim preview has none. In the right
panel the same Pager moves a strip of both tabs (both built only while it moves); the underline and label emphasis
follow its progress (`Tabs::progress`), and clicks, keys and code that set the tab animate through the same spring. A
swipe toward no tab bubbles to the page. Reduced motion: no tracking, the view switches on release with a short fade
(`motion::fade_in_always`).

Rubber band (`ui::elastic`, glue for `motion::interactive::ElasticScroll`): trackpad gestures stretch panes past their
edges with resistance (`physics::rubber_band`), momentum reaching an edge hands its velocity to a `SNAPPY` spring (a
small overshoot that returns), release springs back. The scroll
offset stays in range; the stretch is only a translate of the content (`elastic::content(key)` inside a div scroller
wrapped by `elastic::y_scroller`, a `top` shift of the whole `list` for the diff, a `left` shift inside `hscroll` and
diff rows), so scroll-to-item, the scroll-past-end spacer and keyboard navigation are untouched. Frames come from
`elastic::begin_frame` (window `request_animation_frame` while a spring runs). Mouse wheels (line deltas) and reduced
motion hard-stop at the edges.
