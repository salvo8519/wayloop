"""Generate the Wayloop brand assets (docs/_shared/assets/brand/).

Text is converted to outlines, so the SVGs need no font at display time. The typeface is Inter
(SIL Open Font License), read from the woff2 files of the @fontsource/inter package:

    npm pack @fontsource/inter@5 && tar xzf fontsource-inter-*.tgz
    python scripts/brand.py --fonts package/files

The palette and the rules for using the assets are in docs/en/project/brand.md.
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "_shared" / "assets" / "brand"

INK = "#0B1F3A"
DEEP = "#12325C"
TEAL = "#0D9488"
TEAL_LIGHT = "#2DD4BF"
AMBER = "#F59E0B"
PAPER = "#F7F8FA"
WHITE = "#FFFFFF"
SLATE = "#475569"
NIGHT = "#0A1628"

TAGLINE = "Spec-driven agentic loop."
PROMISE = "From idea to merged code, in your issues, with humans at the gates."


def polar(r: float, deg: float, c: float = 32.0) -> tuple[float, float]:
    a = math.radians(deg)
    return (round(c + r * math.cos(a), 2), round(c + r * math.sin(a), 2))


def mark_body(seal: str, ring: str, gate: str, token: str) -> str:
    """The mark: a loop of work that passes through a gate, on a seal.

    The ring is the loop; the amber diamond sitting in its opening is the human gate, where the
    loop stops for a decision; the teal token is the current step moving along the loop. The
    gate is a diamond, not a bar: a bar across the opening would read as a power symbol.
    """
    r = 15.0
    start, end = polar(r, -20), polar(r, -70)
    gx, gy = polar(r, -45)
    tk = polar(r, 135)
    return (
        f'<rect x="2" y="2" width="60" height="60" rx="14" fill="{seal}"/>'
        f'<path d="M{start[0]} {start[1]} A{r} {r} 0 1 1 {end[0]} {end[1]}" fill="none" '
        f'stroke="{ring}" stroke-width="6" stroke-linecap="round"/>'
        f'<rect x="{gx - 5}" y="{gy - 5}" width="10" height="10" rx="1.6" '
        f'transform="rotate(45 {gx} {gy})" fill="{gate}"/>'
        f'<circle cx="{tk[0]}" cy="{tk[1]}" r="4.6" fill="{token}" stroke="{seal}" '
        f'stroke-width="1.6"/>'
    )


def mark(inverted: bool = False) -> str:
    return (
        mark_body(WHITE, INK, AMBER, TEAL)
        if inverted
        else mark_body(INK, WHITE, AMBER, TEAL_LIGHT)
    )


class Outliner:
    def __init__(self, path: Path) -> None:
        self.font = TTFont(str(path))
        self.glyphs = self.font.getGlyphSet()
        self.cmap = self.font.getBestCmap()
        self.upm = self.font["head"].unitsPerEm
        self.hmtx = self.font["hmtx"]

    def width(self, text: str, size: float, tracking: float = 0.0) -> float:
        total = 0.0
        for ch in text:
            name = self.cmap[ord(ch)]
            total += self.hmtx[name][0] * size / self.upm + tracking
        return total - tracking

    def path(self, text: str, x: float, baseline: float, size: float, tracking: float = 0.0) -> str:
        scale = size / self.upm
        out = []
        cursor = x
        for ch in text:
            name = self.cmap[ord(ch)]
            pen = SVGPathPen(self.glyphs)
            self.glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, cursor, baseline)))
            out.append(pen.getCommands())
            cursor += self.hmtx[name][0] * scale + tracking
        return " ".join(c for c in out if c)


def svg(width: float, height: float, label: str, body: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:g}" height="{height:g}" '
        f'viewBox="0 0 {width:g} {height:g}" role="img" aria-label="{label}">'
        f"<title>{label}</title>{body}</svg>\n"
    )


def placed_mark(x: float, y: float, size: float, inverted: bool = False) -> str:
    return f'<g transform="translate({x:g} {y:g}) scale({size / 64:g})">{mark(inverted)}</g>'


def build(fonts: Path) -> None:
    bold = Outliner(fonts / "inter-latin-700-normal.woff2")
    semi = Outliner(fonts / "inter-latin-600-normal.woff2")
    reg = Outliner(fonts / "inter-latin-400-normal.woff2")
    OUT.mkdir(parents=True, exist_ok=True)

    (OUT / "mark.svg").write_text(svg(64, 64, "Wayloop", mark()), encoding="utf-8")
    (OUT / "mark-inverted.svg").write_text(
        svg(64, 64, "Wayloop", mark(inverted=True)), encoding="utf-8"
    )

    # Horizontal lockup: mark + wordmark.
    size = 34.0
    word_w = bold.width("Wayloop", size, -0.6)
    width = 64 + 18 + word_w + 4
    for name, color in (("logo-light.svg", INK), ("logo-dark.svg", WHITE)):
        body = placed_mark(0, 0, 64) + (
            f'<path d="{bold.path("Wayloop", 82, 44, size, -0.6)}" fill="{color}"/>'
        )
        (OUT / name).write_text(svg(round(width), 64, "Wayloop", body), encoding="utf-8")

    # README banner, 1280 x 320.
    for name, bg, word, sub, rule in (
        ("banner-light.svg", PAPER, INK, SLATE, TEAL),
        ("banner-dark.svg", NIGHT, WHITE, "#A9B8CC", TEAL_LIGHT),
    ):
        wsize, tsize, psize = 76.0, 30.0, 21.0
        block_w = max(
            bold.width("Wayloop", wsize, -1.5),
            semi.width(TAGLINE, tsize),
            reg.width(PROMISE, psize),
        )
        total = 148 + 44 + block_w
        x0 = (1280 - total) / 2
        tx = x0 + 148 + 44
        body = (
            f'<rect width="1280" height="320" rx="16" fill="{bg}"/>'
            + placed_mark(x0, 86, 148)
            + f'<path d="{bold.path("Wayloop", tx, 158, wsize, -1.5)}" fill="{word}"/>'
            + f'<rect x="{tx:.1f}" y="180" width="56" height="4" rx="2" fill="{rule}"/>'
            + f'<path d="{semi.path(TAGLINE, tx, 222, tsize)}" fill="{word}"/>'
            + f'<path d="{reg.path(PROMISE, tx, 256, psize)}" fill="{sub}"/>'
        )
        (OUT / name).write_text(
            svg(1280, 320, f"Wayloop. {TAGLINE} {PROMISE}", body), encoding="utf-8"
        )

    # GitHub social preview, 1280 x 640.
    wsize, tsize, psize = 96.0, 38.0, 25.0
    body = (
        f'<rect width="1280" height="640" fill="{NIGHT}"/>'
        + placed_mark(560, 92, 160)
        + f'<path d="{bold.path("Wayloop", (1280 - bold.width("Wayloop", wsize, -2)) / 2, 382, wsize, -2)}" fill="{WHITE}"/>'
        + f'<rect x="612" y="408" width="56" height="4" rx="2" fill="{TEAL_LIGHT}"/>'
        + f'<path d="{semi.path(TAGLINE, (1280 - semi.width(TAGLINE, tsize)) / 2, 470, tsize)}" fill="{WHITE}"/>'
        + f'<path d="{reg.path(PROMISE, (1280 - reg.width(PROMISE, psize)) / 2, 516, psize)}" fill="#A9B8CC"/>'
    )
    (OUT / "social-preview.svg").write_text(
        svg(1280, 640, f"Wayloop. {TAGLINE}", body), encoding="utf-8"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fonts", type=Path, required=True, help="folder with Inter woff2 files")
    build(parser.parse_args().fonts)


if __name__ == "__main__":
    main()
