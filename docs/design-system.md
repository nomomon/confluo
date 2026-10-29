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

Neutral scale `palette::zinc::Z50 .. Z950` (Tailwind zinc). Accents, used only
where meaning exists:

| Accent | Meaning                     | Semantic token |
|--------|-----------------------------|----------------|
| green  | open, added, passed         | `SUCCESS`      |
| purple | merged                      | `MERGED`       |
| red    | removed, failed, closed     | `DANGER`       |
| amber  | pending, needs you, starred | `WARNING`      |
| blue   | links, focus                | `ACCENT`, `FOCUS_RING` |

Semantic tokens (`ui::theme::*`):

| Token            | Use                                       |
|------------------|-------------------------------------------|
| `BG`             | opaque background, default tint           |
| `SURFACE`        | inputs, buttons, inline code (white 4%)   |
| `SURFACE_HOVER`  | hover fill (white 7%)                     |
| `SURFACE_ACTIVE` | pressed or selected fill (white 10%)      |
| `BORDER`         | default 1px border (white 8%)             |
| `BORDER_SUBTLE`  | card outlines and dividers (white 5%)     |
| `TEXT`           | primary text (zinc 100)                   |
| `TEXT_MUTED`     | secondary text, labels (zinc 400)         |
| `TEXT_SUBTLE`    | hints, timestamps, placeholders (zinc 500)|
| `OVERLAY`        | dialogs and menus (zinc 900, 97%)         |
| `SCRIM`          | modal backdrop (black 45%)                |

`tint_bg(color)` gives the 14% fill used behind pills, badges and notices.

### Translucency

`Translucency::new(panel_opacity, tint, base_alpha)` derives fills from the
appearance settings (`GitViewer::surfaces()` builds it):

- `window_bg()`: the user tint at the appearance's base alpha
  (opaque 1.0, blurred 0.35, transparent 0.55)
- `panel()`: zinc 900 at panel opacity (sidebar, detail)
- `card()`: zinc 800 at 60% of panel opacity (cards, list rows)

## Spacing, radius, type

Spacing, Tailwind 4px steps: `SPACE_0` 0, `SPACE_0P5` 2, `SPACE_1` 4,
`SPACE_1P5` 6, `SPACE_2` 8, `SPACE_2P5` 10, `SPACE_3` 12, `SPACE_4` 16,
`SPACE_5` 20, `SPACE_6` 24, `SPACE_8` 32, `SPACE_10` 40, `SPACE_12` 48.

Icons: `ICON_SM` 12 (chevrons), `ICON_MD` 14 (buttons, nav), `ICON_LG` 16 (row state).
Never use text glyphs as icons; add a Lucide SVG and an `IconName` variant instead.

Radius: `RADIUS_SM` 6, `RADIUS_MD` 8, `RADIUS_LG` 12, `RADIUS_XL` 16,
`RADIUS_FULL`.

Type: `TEXT_2XS` 10 (avatar initials), `TEXT_XS` 12, `TEXT_SM` 13 (body default), `TEXT_BASE` 14, `TEXT_LG` 16,
`TEXT_XL` 20 (detail title), `TEXT_2XL` 26 (page title). Weights:
`WEIGHT_NORMAL`, `WEIGHT_MEDIUM`, `WEIGHT_SEMIBOLD`, `WEIGHT_BOLD`. Fonts:
`UI_FONT` (system), `MONO_FONT` (Menlo) for branches, commands and SHAs.

## Components

Atoms: `Avatar` (initial letter), `Badge`, `Button` (`.icon(IconName::Refresh)`, `.tooltip("Refresh  ⌘R")`),
`Chip`, `Icon` (`Icon::new(IconName::ChevronDown).size(ICON_SM).color(..)`, embedded
Lucide SVGs in `apps/desktop/assets/icons`, ISC license), `NavItem` (`.icon(..)`, `.trailing_on_hover(..)`,
`.on_right_click(..)`), `Pill`, `Swatch`, `tooltip(text)` for any stateful element.

Molecules: `Banner` (one-line dismissible error with an action such as Retry),
`Card`, `CopyField`, `Dialog`, `DropdownMenu` (`.on_dismiss` on outside press),
`EmptyState` (children for extra lines and actions), `Field`, `KeyValueRow`,
`ListSection` (rounded card of rows with a collapsible header and count),
`SegmentedTabs` (`.floating()` pill for bottom tab bars), `TextInput`
(`.multiline()`: Enter inserts a newline, Cmd+Enter submits), `Toast`.

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
| `PANEL`       | 220ms    | ease-out quint | panel entrances                      |
| `PANEL_EXIT`  | 154ms    | ease-out quint | exits, ~70% of entry                 |
| `LIST_INSERT` | 180ms    | ease-out quint | newly inserted list items            |

Helpers wrap GPUI `with_animation` with `Animation::new(d).with_easing(..)`
(easings come from `gpui::{linear, ease_in_out, ease_out_quint}`):

- `fade_in(el, id)`: opacity 0 to 1 over `FADE`
- `slide_in_right(el, id, distance)`: offset from the right plus fade, `PANEL`
- `pop_in(el, id)`: 6px rise plus fade, `FADE` timing with quint ease
- `animate(el, id, motion, animator)`: the generic building block
- `Motion::duration()` / `Motion::animation()` for custom cases

Where it is applied: detail pane slides in on open; dialogs fade the scrim and
pop the panel; dropdown menus pop; status notices fade in. Buttons, chips, nav
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
