"""Render the DMG window background (660x400 points) at 1x and 2x into assets/brand/.

Graphite gradient, a soft accent glow under the app slot, and an arrow from the app slot
to the Applications slot. Slot centers must match scripts/make-dmg.sh.

Usage: uv run --no-project --with cairosvg python scripts/brand/generate_dmg_background.py
(cairosvg needs libcairo: `brew install cairo`; set DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib)
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "assets" / "brand"

W, H = 660, 400
APP = (170, 190)   # app icon slot center (points)
APPS = (490, 190)  # Applications slot center
GRAPHITE_TOP = "#2A2C33"
GRAPHITE_BOT = "#0D0E11"
ACCENT = "#7FA8FF"
INK_SHADE = "#B9BDC7"


def plate(c: tuple[int, int]) -> str:
    """Light rounded plate under a slot: Finder draws icon labels in dark text on picture backgrounds."""
    w, h = 168, 196
    return (f'<rect x="{c[0] - w / 2}" y="{c[1] - 88}" width="{w}" height="{h}" rx="24" fill="#ECEEF3" fill-opacity="0.92"/>'
            f'<rect x="{c[0] - w / 2 + 0.5}" y="{c[1] - 87.5}" width="{w - 1}" height="{h - 1}" rx="23.5" fill="none" stroke="#FFFFFF" stroke-opacity="0.6"/>')


def build_svg() -> str:
    font = "Helvetica Neue, Helvetica, Arial, sans-serif"
    x0, x1, y = APP[0] + 96, APPS[0] - 96, APP[1]
    mid = (x0 + x1) / 2
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{GRAPHITE_TOP}"/>
    <stop offset="1" stop-color="{GRAPHITE_BOT}"/>
  </linearGradient>
  <radialGradient id="glow" cx="{APP[0] / W}" cy="{APP[1] / H}" r="0.42">
    <stop offset="0" stop-color="{ACCENT}" stop-opacity="0.20"/>
    <stop offset="1" stop-color="{ACCENT}" stop-opacity="0"/>
  </radialGradient>
  <linearGradient id="arrow" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0" stop-color="{ACCENT}" stop-opacity="0.25"/>
    <stop offset="1" stop-color="{ACCENT}" stop-opacity="0.85"/>
  </linearGradient>
  <linearGradient id="sheen" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.06"/>
    <stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
  </linearGradient>
</defs>
<rect width="{W}" height="{H}" fill="url(#bg)"/>
<rect width="{W}" height="{H}" fill="url(#glow)"/>
<rect width="{W}" height="{H // 2}" fill="url(#sheen)"/>
{plate(APP)}
{plate(APPS)}
<path d="M {x0} {y} Q {mid} {y - 34} {x1} {y}" fill="none" stroke="url(#arrow)" stroke-width="3" stroke-linecap="round" stroke-dasharray="1 9"/>
<path d="M {x1 - 13} {y - 10} L {x1} {y} L {x1 - 15} {y + 5}" fill="none" stroke="{ACCENT}" stroke-opacity="0.85" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
<text x="{W / 2}" y="{H - 52}" text-anchor="middle" font-family="{font}" font-size="14" fill="{INK_SHADE}" fill-opacity="0.85">Drag Confluo to Applications</text>
</svg>"""


def main() -> None:
    import cairosvg

    svg = build_svg().encode()
    for scale, name in [(1, "dmg-background.png"), (2, "dmg-background@2x.png")]:
        out = BRAND / name
        cairosvg.svg2png(bytestring=svg, write_to=str(out), output_width=W * scale, output_height=H * scale)
        print(f"wrote {out}")


if __name__ == "__main__":
    main()
