#!/usr/bin/env python3
# Copyright (c) 2026 Tymofii Kosovskyi
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
"""Write the Tauri build's icons out as SVG files for the native app.

Sources, all in ../../../ui:
  - app.js `ICON`: the shared set (Lucide, ISC licence), rendered the way its
    `svg()` helper renders them: 24x24, no fill, round caps and joins, width 2.
  - app.js: the few path lists drawn inline (`svg('M… ;M…')`).
  - index.html: the inline <svg> elements (title bar, launcher, settings).

Each file is standalone and single-coloured: a stroke or fill that named a
stylesheet colour becomes `currentColor`, since the native app tints an icon
where it draws it. The settings icon cut its line under each knob by filling
the knob with the background colour; here a mask does the cutting, so it keeps
that look in one colour. Shapes that appear twice are written once.

Run from anywhere: python3 native/app/icons/extract_icons.py

With --verify, nothing is written: each icon as the page drew it (stylesheet
colours resolved, black on white) and each file in icons/n0xis are rendered by
the same renderer (rsvg-convert) and compared pixel by pixel with ImageMagick.
Measured 2026-10-05: 59 of 60 identical; the settings icon differs by 5 of 9216
pixels at 96 px, on the anti-aliased rim where the mask cuts the line. Two
different icons differ by about 2000 pixels, so the check is not blind.
"""
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
UI = HERE.parent.parent.parent / "ui"
OUT = HERE / "n0xis"
NS = 'xmlns="http://www.w3.org/2000/svg"'
LUCIDE_ATTRS = ('width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" '
                'stroke-width="2" stroke-linecap="round" stroke-linejoin="round"')

# The inline index.html icons, in document order: a file name, and a piece of
# markup that must be in it. A changed page fails here instead of misnaming.
HTML_ICONS = [
    ("target", '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/>'),
    ("search", '<circle cx="11" cy="11" r="7"/><path d="M21 21l-4-4"/>'),
    ("live", '<path d="M9 9h6v6H9z"/>'),
    ("settings-sliders", 'cx="9" cy="7" r="2.1"'),
    ("window-minimize", '<path d="M2 6h8"/>'),
    ("window-maximize", '<rect x="2.5" y="2.5" width="7" height="7"/>'),
    ("window-close", '<path d="M3 3l6 6M9 3l-6 6"/>'),
    ("add-widget", '<path d="M17 14v6M14 17h6"/>'),
    ("layout-undo", '<path d="M9 7L4 12l5 5M4 12h11a5 5 0 0 1 5 5v1"/>'),
    ("layout-redo", '<path d="M15 7l5 5-5 5M20 12H9a5 5 0 0 0-5 5v1"/>'),
    ("card-static", '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/>'),
    ("card-both", '<path d="M5 3l14 9-14 9z"/>'),
    ("card-dynamic", '<path d="M9 9h6v6H9z"/>'),
    ("search", '<circle cx="11" cy="11" r="7"/><path d="M21 21l-4-4"/>'),
    ("window-close", '<path d="M3 3l6 6M9 3l-6 6"/>'),
    ("provider-api", '<path d="M4 7l8-4 8 4-8 4z"/>'),
    ("provider-local", '<path d="M8 20h8M12 16v4"/>'),
    ("provider-agent", '<path d="M4 17l6-6-6-6M12 19h8"/>'),
    ("provider-off", '<path d="M5 5l14 14"/>'),
]

# Path lists drawn inline in app.js, by a piece of the list they must match.
INLINE_PATHS = [
    ("legend", "M8 6h13;M8 12h13;M8 18h13"),
    ("edge-curve", "M4 19 C 9 5, 15 5, 20 19"),
    ("fit", "M4 9V5a1 1 0 0 1 1-1h4"),
    ("scope", "M12 2a10 10 0 1 0 0 20a10 10 0 0 0 0-20"),
]


def lucide(inner: str) -> str:
    return f"<svg {NS} {LUCIDE_ATTRS}>{inner}</svg>\n"


def single_colour(svg: str) -> str:
    """Stylesheet colours become currentColor; the namespace is added."""
    svg = re.sub(r'(stroke|fill)="var\(--[a-z0-9-]+\)"', r'\1="currentColor"', svg)
    if "xmlns=" not in svg:
        svg = svg.replace("<svg ", f"<svg {NS} ", 1)
    return svg


def knockout(svg: str) -> str:
    """The settings icon: lines cut under each knob by a mask, knobs drawn open."""
    knobs = re.findall(r'<circle cx="([\d.]+)" cy="([\d.]+)" r="([\d.]+)" fill="var\(--bg1\)"/>', svg)
    if not knobs:
        sys.exit("settings-sliders: the knobs are no longer background-filled circles; look at the page")
    lines = "".join(re.findall(r"<line [^>]*/>", svg))
    holes = "".join(f'<circle cx="{x}" cy="{y}" r="{float(r) + 1:g}" fill="black" stroke="none"/>' for x, y, r in knobs)
    rings = "".join(f'<circle cx="{x}" cy="{y}" r="{r}"/>' for x, y, r in knobs)
    head = re.match(r"<svg [^>]*>", svg).group(0)
    body = (f'<mask id="cut"><rect width="24" height="24" fill="white" stroke="none"/>{holes}</mask>'
            f'<g mask="url(#cut)">{lines}</g>{rings}')
    return single_colour(head + body + "</svg>")


def shape_key(svg: str) -> str:
    """What an icon looks like, ignoring the size it was drawn at and its colour."""
    s = re.sub(r'\s(width|height)="\d+"', "", svg.strip(), count=2)
    return re.sub(r'(stroke|fill)="[^"]*"', "", s)


def originals(app_js: str, html: str) -> dict[str, str]:
    """Each icon's markup as the page draws it, keyed by the file name it is written to."""
    found: dict[str, str] = {}
    table = re.search(r"const ICON = \{(.*?)\n\};", app_js, re.S).group(1)
    for name, inner in re.findall(r"^\s+(\w+): \"(.*)\",$", table, re.M):
        found[name.replace("_", "-")] = lucide(inner)
    for name, piece in INLINE_PATHS:
        d = next(m for m in re.findall(r"svg\('([^']*)'", app_js) if piece in m)
        found[name] = lucide("".join(f'<path d="{p}"/>' for p in d.split(";")))
    inline = [re.sub(r"\s+", " ", m) for m in re.findall(r"<svg\b.*?</svg>", html, re.S)]
    for svg, (name, _) in zip(inline, HTML_ICONS):
        found.setdefault(name, svg.replace("<svg ", f"<svg {NS} ", 1))
    return found


def render(svg: str, png: Path) -> None:
    """Black on white at 96 px; a knob filled with the page background is white."""
    svg = re.sub(r'(stroke|fill)="var\(--bg1\)"', r'\1="white"', svg)
    svg = re.sub(r'(stroke|fill)="var\(--[a-z0-9-]+\)"', r'\1="black"', svg).replace("currentColor", "black")
    svg = re.sub(r'\swidth="\d+" height="\d+"', ' width="96" height="96"', svg, count=1)
    subprocess.run(["rsvg-convert", "-b", "white", "-o", str(png)], input=svg.encode(), check=True)


def verify() -> None:
    app_js, html = (UI / "app.js").read_text(), (UI / "index.html").read_text()
    page = originals(app_js, html)
    files = sorted(OUT.glob("*.svg"))
    if sorted(f.stem for f in files) != sorted(page):
        sys.exit(f"the files and the page disagree on which icons exist: {sorted(set(page) ^ {f.stem for f in files})}")
    differing = []
    with tempfile.TemporaryDirectory() as tmp:
        for f in files:
            a, b = Path(tmp) / "a.png", Path(tmp) / "b.png"
            render(page[f.stem], a)
            render(f.read_text(), b)
            out = subprocess.run(["magick", "compare", "-metric", "AE", "-fuzz", "10%", str(a), str(b), "null:"],
                                 capture_output=True, text=True)
            pixels = float(out.stderr.split()[0])
            if pixels > 0:
                differing.append(f"{f.stem} ({pixels:g} px)")
    print(f"{len(files)} icons compared with the page; differing: {', '.join(differing) or 'none'}")


def main() -> None:
    if "--verify" in sys.argv[1:]:
        verify()
        return
    app_js = (UI / "app.js").read_text()
    html = (UI / "index.html").read_text()
    icons: dict[str, str] = {}

    table = re.search(r"const ICON = \{(.*?)\n\};", app_js, re.S)
    if not table:
        sys.exit("app.js has no ICON table")
    for name, inner in re.findall(r"^\s+(\w+): \"(.*)\",$", table.group(1), re.M):
        icons[name] = lucide(inner.replace(" /> <", " /><"))

    for name, piece in INLINE_PATHS:
        found = [m for m in re.findall(r"svg\('([^']*)'", app_js) if piece in m]
        if len(found) != 1:
            sys.exit(f"{name}: expected one inline path list holding {piece!r}, found {len(found)}")
        icons[name] = lucide("".join(f'<path d="{d}"/>' for d in found[0].split(";")))

    inline = [re.sub(r"\s+", " ", m) for m in re.findall(r"<svg\b.*?</svg>", html, re.S)]
    if len(inline) != len(HTML_ICONS):
        sys.exit(f"index.html has {len(inline)} inline icons, the table names {len(HTML_ICONS)}; update the table")
    aliases = []
    for svg, (name, piece) in zip(inline, HTML_ICONS):
        if piece not in svg:
            sys.exit(f"{name}: expected {piece!r} in {svg[:120]}")
        svg = knockout(svg) if 'fill="var(--bg1)"' in svg else single_colour(svg)
        if name in icons:
            if shape_key(icons[name]) != shape_key(svg):
                sys.exit(f"{name}: two different shapes under one name")
            aliases.append(name)
            continue
        icons[name] = svg + "\n"

    OUT.mkdir(exist_ok=True)
    for old in OUT.glob("*.svg"):
        old.unlink()
    for name, svg in sorted(icons.items()):
        (OUT / f"{name.replace('_', '-')}.svg").write_text(svg)
    print(f"{len(icons)} icons written to {OUT.relative_to(HERE.parent)}; drawn twice on the page, written once: {', '.join(aliases)}")


if __name__ == "__main__":
    main()
