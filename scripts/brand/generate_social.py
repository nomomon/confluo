"""Render the GitHub social preview (1280x640) from assets/brand/icon.svg.

Usage: python scripts/brand/generate_social.py  (needs cairosvg)
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "assets" / "brand"

W, H = 1280, 640
ICON = 360


def build_svg() -> str:
    icon = (BRAND / "icon.svg").read_text()
    icon = re.sub(r"<\?xml[^>]*\?>", "", icon)
    icon = re.sub(
        r"<svg ([^>]*?)width=\"1024\" height=\"1024\"",
        rf'<svg \1x="120" y="{(H - ICON) // 2}" width="{ICON}" height="{ICON}"',
        icon,
        count=1,
    )
    font = "Helvetica Neue, Helvetica, Arial, sans-serif"
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#17181C"/>
    <stop offset="1" stop-color="#0B0C0E"/>
  </linearGradient>
  <radialGradient id="glow" cx="0.24" cy="0.5" r="0.45">
    <stop offset="0" stop-color="#7FA8FF" stop-opacity="0.16"/>
    <stop offset="1" stop-color="#7FA8FF" stop-opacity="0"/>
  </radialGradient>
</defs>
<rect width="{W}" height="{H}" fill="url(#bg)"/>
<rect width="{W}" height="{H}" fill="url(#glow)"/>
{icon}
<text x="560" y="300" font-family="{font}" font-size="104" font-weight="600" fill="#F4F5F7" letter-spacing="-2">Confluo</text>
<text x="564" y="372" font-family="{font}" font-size="36" fill="#9DA1AC">Every pull request, one calm place.</text>
<text x="564" y="424" font-family="{font}" font-size="26" fill="#6B6F7A">GitHub · GitLab · Azure DevOps</text>
</svg>"""


def main() -> None:
    import cairosvg

    svg = build_svg()
    (BRAND / "social.svg").write_text(svg)
    cairosvg.svg2png(bytestring=svg.encode(), write_to=str(BRAND / "social.png"), output_width=W, output_height=H)
    print(BRAND / "social.png")


if __name__ == "__main__":
    main()
