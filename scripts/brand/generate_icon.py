#!/usr/bin/env python3
"""Generate the Confluo app icon.

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


def glow(x: float, y: float, r: float) -> str:
    return f'<circle cx="{x:.2f}" cy="{y:.2f}" r="{r}" fill="url(#glow)"/>'


# ------------------------------------------------------------------- glyph
def glyph() -> str:
    """Three tributaries meet tangent-continuous at a rotated-square hub (git nod);
    the river leaves upward with an open end (no terminal node: it keeps flowing).

    Each feeder is a cubic whose end tangent is vertical, so the three
    strands merge into the trunk like streams, not like a tree diagram.
    """
    w = 0.625 * U                      # 40px: bold enough to survive 16px
    hub = (C, C - 0.5 * U)
    top_y = C - 3.5 * U
    srcs = [(C - 3 * U, C + 3 * U), (C, C + 3 * U), (C + 3 * U, C + 3 * U)]
    r = 1.5 * U
    land = (hub[0], hub[1] + r * 0.75)  # streams land on the hub's lower vertex
    g = glow(*hub, 2.75 * U)
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
    hx, hy = hub
    d = f"M{hx},{hy - r} L{hx + r},{hy} L{hx},{hy + r} L{hx - r},{hy} Z"
    g += f'<path d="{d}" fill="url(#ink)" stroke="url(#ink)" stroke-width="{0.25 * U}" stroke-linejoin="round"/>'
    return g


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="assets/brand")
    ap.add_argument("--png", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    svg = frame(glyph())
    (out / "icon.svg").write_text(svg)
    if not args.png:
        return
    import cairosvg

    cairosvg.svg2png(bytestring=svg.encode(), write_to=str(out / "icon-1024.png"),
                     output_width=CANVAS, output_height=CANVAS)

    # iconset
    iconset = out / "AppIcon.iconset"
    iconset.mkdir(exist_ok=True)
    for s in (16, 32, 64, 128, 256, 512):
        for scale in (1, 2):
            px = s * scale
            if px > 1024:
                continue
            nm = f"icon_{s}x{s}{'@2x' if scale == 2 else ''}.png"
            png = cairosvg.svg2png(bytestring=svg.encode(), output_width=px, output_height=px)
            (iconset / nm).write_bytes(png)


if __name__ == "__main__":
    main()
