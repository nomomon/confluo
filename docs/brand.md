# Brand

## Name: Confluo

Latin for "I flow together", the root of *confluence*: the point where
streams meet and continue as one river. That is the product: pull and merge
requests from GitHub, GitLab and Azure DevOps flow into one calm place.

- Short, two to three syllables (con-FLOO-oh), easy to say in English and Dutch.
- Quiet meaning: convergence without a hierarchy; no provider "wins".
- Collisions checked (`gh search repos`): only `ucbrise/confluo`, an older
  research database, not a developer tool in the same space. "Confluence"
  itself is taken by Atlassian, which is why the Latin verb is used.

Candidates considered:

| Name | Verdict |
| --- | --- |
| Confluo | Chosen |
| Inflow | Runner-up: clear, but generic and used by several small projects |
| Skein | Runner-up: a bundle of threads; taken by a Bevy/Rust tool |
| Tributary | Long; `enjalot/tributary` is a known code tool |
| Estuary | Estuary Flow is a known data product |
| Weir | Nice and short, but a weir holds water back (wrong metaphor) |
| Braid | Several established projects (braid design system, braidchat) |
| Delta | Collides with `git-delta`, a popular git diff tool |
| Conflux | Conflux blockchain |
| Rill | Rill Data BI tool, `destel/rill` |
| Fjord / Lagoon | Taken by launchers and hosting platforms |

## Icon design rules

Canvas and body follow the macOS Big Sur template.

- **Canvas:** 1024 x 1024. **Body:** 824 x 824 superellipse (n = 4.4), offset 100.
  Equivalent corner radius is roughly 0.225 of the body size.
- **Grid:** 8-unit grid inside a 512 px safe area, `u = 64 px`, centred.
  Every coordinate is a multiple of 0.25 u.
- **Strokes:** primary stroke 0.625 u (40 px, 1/12.8 of the body), round caps
  and joins. Secondary strokes are 0.75x or 0.5x of the primary, never thinner.
- **Nodes:** source nodes 0.6875 u radius; the hub is the heaviest shape
  (rotated square, half-diagonal 1.5 u, corners rounded by a 0.25 u stroke).
- **Curves:** cubic beziers only, tangent-continuous at the hub (vertical
  in, vertical out); control points at 0.75 h and 0.618 h of the rise.
- **Palette (max 3):** graphite body `#2A2C33 -> #0D0E11`, ink `#F4F5F7 -> #B9BDC7`,
  one cool accent `#7FA8FF` used only as light (glow, never as fill).
- **Light:** from the top. Body gradient darkens downward, a top sheen at
  10% white, a 3 px rim highlight strongest at the top edge, drop shadow
  12 px down, 14 px blur, 45% black.
- **Effects allowed:** subtle linear/radial gradients, inner rim highlight,
  one soft accent glow behind the hub. No noise, no textures, no bevels, no
  multi-color fills.
- **Legibility:** the silhouette (trunk, hub, three roots) must survive 16 px;
  anything that does not read at 32 px is decoration and must go.

## Concepts explored

Seven concepts (confluence, diamond, prism, trefoil, delta, merge glyph,
confluence-diamond) were compared at 1024, 32 and 16 px. The final is the
confluence with git's rotated square as the hub, streams landing on its lower
vertex. The others failed at 16 px, read as git's logo, or were too generic.
Sources live in git history (`git log -- assets/brand/concepts`).

## Files

- `scripts/brand/generate_icon.py` - generator (`--png` needs `cairosvg`, `pillow`)
- `assets/brand/icon.svg`, `assets/brand/icon-1024.png` - final icon
- `assets/brand/AppIcon.icns` - built with `iconutil` from a 16 to 1024 iconset
