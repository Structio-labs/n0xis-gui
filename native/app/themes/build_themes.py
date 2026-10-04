#!/usr/bin/env python3
# Copyright (c) 2026 Tymofii Kosovskyi
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
"""Build n0xis.json, the native GUI's built-in themes, from the Tauri UI's own
palette (../../../ui/styles.css), so the six themes users already know keep
their exact colours. Every token is read from the stylesheet, never retyped.

    python3 native/app/themes/build_themes.py      # rewrites n0xis.json next to it
"""
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
CSS = HERE.parents[2] / "ui" / "styles.css"
OUT = HERE / "n0xis.json"

# Display name and stylesheet selector of each theme, in the order the old
# settings grid showed them. Midnight is the stylesheet's :root.
THEMES = [
    ("Midnight", ":root"),
    ("Deep", '[data-theme="deep"]'),
    ("Light", '[data-theme="light"]'),
    ("Nebula", '[data-theme="nebula"]'),
    ("Warm", '[data-theme="warm"]'),
    ("Forest", '[data-theme="forest"]'),
]
LIGHT = {"Light"}


def blocks(css):
    """selector -> {token: value} for every block that defines custom properties."""
    out = {}
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)  # comments would read as part of a selector
    for sel, body in re.findall(r"([^{}]+)\{([^{}]*)\}", css):
        tokens = dict(re.findall(r"--([a-z0-9]+)\s*:\s*([^;]+);", body))
        if tokens:
            # A selector is what follows the last statement before the brace
            # (an @import line ends with `;` and would otherwise stick to it).
            selector = sel.split(";")[-1].strip()
            out.setdefault(selector, {}).update({k: v.strip() for k, v in tokens.items()})
    return out


def palette(all_blocks, selector):
    """A theme's tokens: the root defaults, overridden by the theme's own block."""
    tokens = dict(all_blocks[":root"])
    if selector != ":root":
        tokens.update(all_blocks[selector])
    return tokens


def theme(name, t):
    dark = name not in LIGHT
    clear = lambda c: c + "00"
    tint = lambda c, alpha: c + alpha
    colors = {
        "background": t["bg1"],
        "foreground": t["tx0"],
        "border": t["bd"],
        "input.border": t["bd2"],
        "muted.background": t["bg2"],
        "muted.foreground": t["tx2"],
        "accent.background": t["hov"],
        "accent.foreground": t["tx0"],
        "primary.background": t["acc"],
        "primary.foreground": t["bg0"] if dark else t["bg2"],
        "primary.hover.background": tint(t["acc"], "dd"),
        "primary.active.background": t["accd"],
        "secondary.background": t["bg2"],
        "secondary.foreground": t["tx1"],
        "secondary.hover.background": t["hov"],
        "secondary.active.background": t["bg3"],
        "popover.background": t["bg2"],
        "popover.foreground": t["tx0"],
        "list.background": t["bg1"],
        "list.even.background": t["bg1"],
        "list.head.background": t["bg2"],
        "list.hover.background": t["hov"],
        "list.active.background": tint(t["acc"], "26"),
        "list.active.border": tint(t["acc"], "88"),
        "selection.background": tint(t["acc"], "40"),
        "caret": t["acc"],
        "ring": t["acc"],
        "link": t["acc"],
        "link.hover": t["acc"],
        "link.active": t["accd"],
        "danger.background": t["dgr"],
        "danger.foreground": t["bg0"] if dark else t["bg2"],
        "success.background": t["ok"],
        "success.foreground": t["bg0"] if dark else t["bg2"],
        "warning.background": t["live"],
        "warning.foreground": t["bg0"] if dark else t["bg2"],
        "info.background": t["fn"],
        "info.foreground": t["bg0"] if dark else t["bg2"],
        "title_bar.background": t["bg0"],
        "title_bar.border": t["bd"],
        "status_bar.background": t["bg0"],
        "status_bar.border": t["bd"],
        "tab_bar.background": t["bg0"],
        "tab.background": clear(t["bg0"]),
        "tab.foreground": t["tx1"],
        "tab.active.background": t["bg1"],
        "tab.active.foreground": t["tx0"],
        "scrollbar.background": clear(t["bg0"]),
        "scrollbar.thumb.background": t["bd2"],
        "scrollbar.thumb.hover.background": t["tx2"],
        "window.border": t["bd"],
        "drop_target.background": tint(t["acc"], "26"),
        "drag.border": t["acc"],
        "base.red": t["dgr"],
        "base.green": t["ok"],
        "base.yellow": t["live"],
        "base.blue": t["fn"],
        "base.magenta": t["vio"],
        "base.cyan": t["acc"],
    }
    syntax = {
        "keyword": {"color": t["k"]},
        "preproc": {"color": t["k"]},
        "type": {"color": t["ty"]},
        "function": {"color": t["fn"]},
        "string": {"color": t["st"]},
        "string.escape": {"color": t["st"]},
        "number": {"color": t["nu"]},
        "constant": {"color": t["nu"]},
        "boolean": {"color": t["nu"]},
        "comment": {"color": t["cm"], "font_style": "italic"},
        "comment.doc": {"color": t["cm"], "font_style": "italic"},
        "punctuation": {"color": t["pu"]},
        "punctuation.bracket": {"color": t["pu"]},
        "punctuation.delimiter": {"color": t["pu"]},
        "operator": {"color": t["pu"]},
        "property": {"color": t["tx0"]},
        "variable": {"color": t["tx0"]},
        "label": {"color": t["vio"]},
    }
    highlight = {
        "editor.background": t["bg1"],
        "editor.foreground": t["tx0"],
        "editor.active_line.background": t["bg2"],
        "editor.line_number": t["tx2"],
        "editor.active_line_number": t["tx1"],
        "editor.invisible": tint(t["tx2"], "66"),
        "error": t["dgr"],
        "warning": t["live"],
        "info": t["fn"],
        "hint": t["vio"],
        "success": t["ok"],
        "syntax": syntax,
    }
    return {"name": name, "mode": "dark" if dark else "light", "colors": colors, "highlight": highlight}


def main():
    all_blocks = blocks(CSS.read_text())
    themes = [theme(name, palette(all_blocks, sel)) for name, sel in THEMES]
    doc = {"name": "N0xis", "author": "N0xis", "themes": themes}
    OUT.write_text(json.dumps(doc, indent=2) + "\n")
    print(f"wrote {OUT} ({len(themes)} themes)")


if __name__ == "__main__":
    main()
