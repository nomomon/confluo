#!/usr/bin/env python3
"""Generate the Confluo app icon and concept explorations.

All geometry is computed here: a superellipse body on a 1024 canvas and
glyphs built on an 8-unit grid (u = 64px) inside a 512px safe area.

Usage:
    python scripts/brand/generate_icon.py [--out assets/brand] [--png]

--png needs cairosvg + Pillow (pip install cairosvg pillow).
"""
from __future__ import annotations

import argparse
import math
from pathlib import Path

# ---------------------------------------------------------------- tokens
CANVAS = 1024
BODY = 824                       # macOS Big Sur template body
BODY_X = (CANVAS - BODY) / 2     # 100
SQUIRCLE_N = 4.4                 # superellipse exponent (continuous corners)
U = 64                           # grid unit; safe area = 8u = 512
C = CANVAS / 2                   # centre
STROKE = 0.5 * U                 # 32px primary stroke
NODE_R = 0.75 * U                # 48px node radius
HUB_R = 1.0 * U                  # 64px merge-node radius

GRAPHITE_TOP = "#2A2C33"
GRAPHITE_BOT = "#0D0E11"
INK = "#F4F5F7"
INK_SHADE = "#B9BDC7"
ACCENT = "#7FA8FF"               # single cool accent, used only as light


def squircle_path(x: float, y: float, size: float, n: float = SQUIRCLE_N, steps: int = 2048) -> str:
    a = size / 2
    cx, cy = x + a, y + a
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        ct, st = math.cos(t), math.sin(t)
        px = cx + a * math.copysign(abs(ct) ** (2 / n), ct)
        py = cy + a * math.copysign(abs(st) ** (2 / n), st)
        pts.append(f"{px:.2f},{py:.2f}")
    return "M" + " L".join(pts) + " Z"


DEFS = f"""
<defs>
  <linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{GRAPHITE_TOP}"/>
    <stop offset="1" stop-color="{GRAPHITE_BOT}"/>
  </linearGradient>
  <radialGradient id="sheen" cx="0.5" cy="0.0" r="0.9">
    <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.10"/>
    <stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
  </radialGradient>
  <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.28"/>
    <stop offset="0.5" stop-color="#FFFFFF" stop-opacity="0.04"/>
    <stop offset="1" stop-color="#FFFFFF" stop-opacity="0.10"/>
  </linearGradient>
  <linearGradient id="ink" gradientUnits="userSpaceOnUse" x1="0" y1="256" x2="0" y2="768">
    <stop offset="0" stop-color="{INK}"/>
    <stop offset="1" stop-color="{INK_SHADE}"/>
  </linearGradient>
  <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
    <stop offset="0" stop-color="{ACCENT}" stop-opacity="0.55"/>
    <stop offset="1" stop-color="{ACCENT}" stop-opacity="0"/>
  </radialGradient>
  <filter id="shadow" filterUnits="userSpaceOnUse" x="0" y="0" width="1024" height="1024">
    <feGaussianBlur in="SourceAlpha" stdDeviation="14"/>
    <feOffset dy="12"/>
    <feComponentTransfer><feFuncA type="linear" slope="0.45"/></feComponentTransfer>
    <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
</defs>"""


def frame(glyph: str) -> str:
    sq = squircle_path(BODY_X, BODY_X, BODY)
    inner = squircle_path(BODY_X + 1.5, BODY_X + 1.5, BODY - 3)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{CANVAS}" height="{CANVAS}" viewBox="0 0 {CANVAS} {CANVAS}">
{DEFS}
<g filter="url(#shadow)"><path d="{sq}" fill="url(#body)"/></g>
<path d="{sq}" fill="url(#sheen)"/>
<path d="{inner}" fill="none" stroke="url(#rim)" stroke-width="3"/>
<g>{glyph}</g>
</svg>"""


def stroke(d: str, w: float = STROKE, color: str = "url(#ink)") -> str:
    return f'<path d="{d}" fill="none" stroke="{color}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"/>'


def dot(x: float, y: float, r: float = NODE_R, fill: str = "url(#ink)") -> str:
    return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r}" fill="{fill}"/>'


def ring(x: float, y: float, r: float = NODE_R, w: float = STROKE) -> str:
    return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r - w / 2}" fill="none" stroke="url(#ink)" stroke-width="{w}"/>'


def glow(x: float, y: float, r: float) -> str:
    return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r}" fill="url(#glow)"/>'


# ---------------------------------------------------------------- concepts
def concept_confluence(frame_diamond: bool = False) -> str:
    """Three tributaries meet tangent-continuous at one hub; the river leaves
    upward with an open end (no terminal node: it keeps flowing).

    Grid: sources on y = +2.5u, spaced 2.5u; hub at y = -0.5u; trunk to -3.25u.
    Each feeder is a cubic whose end tangent is vertical, so the three
    strands merge into the trunk like streams, not like a tree diagram.
    """
    w = 0.625 * U                      # 40px: bold enough to survive 16px
    hub = (C, C - 0.5 * U)
    top_y = C - 3.5 * U
    srcs = [(C - 3 * U, C + 3 * U), (C, C + 3 * U), (C + 3 * U, C + 3 * U)]
    g = ""
    g += glow(*hub, 2.75 * U)
    # with the diamond hub, streams land on its lower vertex instead of its centre
    land = (hub[0], hub[1] + 1.5 * U * 0.75) if frame_diamond else hub
    for sx, sy in srcs:
        if sx == C:
            d = f"M{sx},{sy} L{hub[0]},{hub[1]}"
        else:
            # leave the source vertically, arrive at the hub vertically (golden split of the height)
            h = sy - land[1]
            d = (f"M{sx},{sy} C{sx},{sy - h * 0.75} "
                 f"{land[0]},{land[1] + h * 0.618} {land[0]},{land[1]}")
        g += stroke(d, w)
    g += stroke(f"M{hub[0]},{hub[1]} L{C},{top_y}", w)
    for s in srcs:
        g += dot(*s, 0.6875 * U)       # 44px source nodes
    if frame_diamond:
        # git nod: the hub is the rotated square, corners rounded by 1/8u
        r = 1.5 * U
        hx, hy = hub
        d = f"M{hx},{hy - r} L{hx + r},{hy} L{hx},{hy + r} L{hx - r},{hy} Z"
        g += f'<path d="{d}" fill="url(#ink)" stroke="url(#ink)" stroke-width="{0.25 * U}" stroke-linejoin="round"/>'
    else:
        g += dot(*hub, 0.9375 * U)     # 60px hub
    return g


def concept_confluence_diamond() -> str:
    return concept_confluence(frame_diamond=True)


def concept_diamond() -> str:
    """Git's rotated square; three branch lines enter the lower half, merge at the centre node."""
    s = 3.5 * U
    pts = [(C, C - s), (C + s, C), (C, C + s), (C - s, C)]
    d = "M" + " L".join(f"{x},{y}" for x, y in pts) + " Z"
    g = stroke(d, STROKE * 0.75)
    g += glow(C, C, 2 * U)
    g += stroke(f"M{C},{C + s} L{C},{C - s}")
    g += stroke(f"M{C - s / 2},{C + s / 2} L{C},{C}")
    g += stroke(f"M{C + s / 2},{C + s / 2} L{C},{C}")
    g += dot(C, C, HUB_R * 0.9)
    return g


def concept_prism() -> str:
    """Equilateral prism: three rays enter one face, a single beam leaves."""
    r = 2.4 * U
    tri = [(C + r * math.cos(math.radians(a)), C + 0.2 * U + r * math.sin(math.radians(a))) for a in (-90, 30, 150)]
    d = "M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in tri) + " Z"
    g = ""
    lx = tri[2][0] + (tri[0][0] - tri[2][0]) * 0.5
    ly = tri[2][1] + (tri[0][1] - tri[2][1]) * 0.5
    for k, off in enumerate((-0.6, 0, 0.6)):
        g += stroke(f"M{C - 4 * U},{ly + off * U * 1.6} L{lx - 0.1 * U},{ly + off * U * 0.5}", STROKE * 0.5)
    g += glow(C + 0.3 * U, C, 2.2 * U)
    g += f'<path d="{d}" fill="url(#ink)" fill-opacity="0.14" stroke="url(#ink)" stroke-width="{STROKE * 0.75}" stroke-linejoin="round"/>'
    rx = tri[1][0] - (tri[1][0] - tri[0][0]) * 0.5
    ry = tri[1][1] - (tri[1][1] - tri[0][1]) * 0.5
    g += stroke(f"M{rx},{ry} L{C + 4 * U},{ry}", STROKE)
    return g


def concept_trefoil() -> str:
    """Three strands, 120 degree rotational symmetry, woven around one centre."""
    g = glow(C, C, 2.5 * U)
    R = 2.0 * U
    for k in range(3):
        a = math.radians(-90 + 120 * k)
        cx, cy = C + R * 0.55 * math.cos(a), C + R * 0.55 * math.sin(a)
        g += f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{R}" fill="none" stroke="url(#ink)" stroke-width="{STROKE * 0.75}"/>'
    g += dot(C, C, NODE_R * 0.8)
    return g


def concept_delta() -> str:
    """A river delta inverted: three nodes on a triangle drain into the apex."""
    apex = (C, C - 3 * U)
    base = [(C - 3 * U, C + 2.5 * U), (C, C + 2.5 * U), (C + 3 * U, C + 2.5 * U)]
    g = glow(*apex, 2.2 * U)
    for x, y in base:
        g += stroke(f"M{x},{y} L{apex[0]},{apex[1]}", STROKE * 0.75)
    for b in base:
        g += ring(*b, NODE_R * 0.9, STROKE * 0.75)
    g += dot(*apex, HUB_R)
    return g


def concept_merge() -> str:
    """Merge glyph on the 8u grid. Two branches arc into the trunk,
    one ring (open MR) at the source, one solid hub where everything lands."""
    trunk_x = C - 1.0 * U
    top, bot = C - 3.0 * U, C + 3.0 * U
    hub_y = C - 1.0 * U
    g = glow(trunk_x, hub_y, 2.6 * U)
    # trunk
    g += stroke(f"M{trunk_x},{bot} L{trunk_x},{top}")
    # two branches, quarter-ellipse arcs into the hub (k = 0.5523 bezier circle constant)
    k = 0.5523
    bx, by = C + 2.0 * U, C + 2.0 * U
    d = (f"M{bx},{by} L{bx},{hub_y + 1.5 * U} "
         f"C{bx},{hub_y + 1.5 * U - k * 1.5 * U} {bx - 3 * U + k * 3 * U},{hub_y} {trunk_x},{hub_y}")
    g += stroke(d)
    g += ring(bx, by, NODE_R)
    g += dot(trunk_x, top, NODE_R * 0.8)
    g += dot(trunk_x, bot, NODE_R * 0.8)
    g += dot(trunk_x, hub_y, HUB_R)
    return g


CONCEPTS = {
    "a-confluence": concept_confluence,
    "b-diamond": concept_diamond,
    "c-prism": concept_prism,
    "d-trefoil": concept_trefoil,
    "e-delta": concept_delta,
    "f-merge": concept_merge,
    "g-confluence-diamond": concept_confluence_diamond,
}
FINAL = "g-confluence-diamond"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="assets/brand")
    ap.add_argument("--png", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    (out / "concepts").mkdir(parents=True, exist_ok=True)
    svgs = {}
    for name, fn in CONCEPTS.items():
        svgs[name] = frame(fn())
        (out / "concepts" / f"{name}.svg").write_text(svgs[name])
    (out / "icon.svg").write_text(svgs[FINAL])
    if not args.png:
        return
    import io
    import cairosvg
    from PIL import Image, ImageDraw

    imgs = {}
    for name, svg in svgs.items():
        png = cairosvg.svg2png(bytestring=svg.encode(), output_width=CANVAS, output_height=CANVAS)
        imgs[name] = Image.open(io.BytesIO(png)).convert("RGBA")
    imgs[FINAL].save(out / "icon-1024.png")

    # contact sheet: each concept at 256 plus 32 and 16 previews
    cell = 300
    sheet = Image.new("RGBA", (cell * 3, cell * ((len(imgs) + 2) // 3)), "#18191D")
    draw = ImageDraw.Draw(sheet)
    for i, (name, im) in enumerate(imgs.items()):
        x, y = (i % 3) * cell, (i // 3) * cell
        sheet.alpha_composite(im.resize((240, 240), Image.LANCZOS), (x + 10, y + 10))
        sheet.alpha_composite(im.resize((32, 32), Image.LANCZOS), (x + 254, y + 40))
        sheet.alpha_composite(im.resize((16, 16), Image.LANCZOS), (x + 262, y + 90))
        draw.text((x + 14, y + 262), name + (" (final)" if name == FINAL else ""), fill="#C9CDD6")
    sheet.save(out / "contact-sheet.png")

    # iconset
    iconset = out / "AppIcon.iconset"
    iconset.mkdir(exist_ok=True)
    for s in (16, 32, 64, 128, 256, 512):
        for scale in (1, 2):
            px = s * scale
            if px > 1024:
                continue
            nm = f"icon_{s}x{s}{'@2x' if scale == 2 else ''}.png"
            png = cairosvg.svg2png(bytestring=svgs[FINAL].encode(), output_width=px, output_height=px)
            (iconset / nm).write_bytes(png)


if __name__ == "__main__":
    main()
