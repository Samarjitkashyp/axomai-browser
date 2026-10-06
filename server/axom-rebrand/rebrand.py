# -*- coding: utf-8 -*-
"""Gives the Axom AI Next.js site (aiaxom.co.in) the colours and the font of the Axomai Browser site.

Run it on a COPY of next-frontend, never on the folder that is being served:
    python3 rebrand.py /path/to/copy-of/next-frontend

What it does
  * Colours: the purple / violet / pink / fuchsia / indigo scales (Tailwind classes, hex values and rgba values) become the
    Axomai Browser palette: emerald (#10b981 / #059669) with amber (#f59e0b) as the second colour, teal for the old indigo.
    The near-black purple backgrounds become the Axomai dark navy (#0b1220, cards #111c2e).
  * Font: Inter becomes Plus Jakarta Sans, Instrument Serif becomes Plus Jakarta Sans, Noto Serif Bengali becomes Noto Sans Bengali.
Everything else (layout, text, other colours such as red for errors or blue/cyan) is left alone.
"""
import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else '.'
EXT = ('.tsx', '.ts', '.jsx', '.js', '.css', '.mjs')
SKIP_DIRS = {'node_modules', '.next', '.git', 'public'}

# ---- hex colours (old -> new), lower case
HEX = {}


def scale(old, new):
    for o, n in zip(old, new):
        HEX[o] = n


emerald = ['#ecfdf5', '#d1fae5', '#a7f3d0', '#6ee7b7', '#34d399', '#10b981', '#059669', '#047857', '#065f46', '#064e3b']
amber = ['#fffbeb', '#fef3c7', '#fde68a', '#fcd34d', '#fbbf24', '#f59e0b', '#d97706', '#b45309', '#92400e', '#78350f']
teal = ['#f0fdfa', '#ccfbf1', '#99f6e4', '#5eead4', '#2dd4bf', '#14b8a6', '#0d9488', '#0f766e', '#115e59', '#134e4a']
# shades 50 100 200 300 400 500 600 700 800 900
scale(['#faf5ff', '#f3e8ff', '#e9d5ff', '#d8b4fe', '#c084fc', '#a855f7', '#9333ea', '#7e22ce', '#6b21a8', '#581c87'], emerald)      # purple
scale(['#f5f3ff', '#ede9fe', '#ddd6fe', '#c4b5fd', '#a78bfa', '#8b5cf6', '#7c3aed', '#6d28d9', '#5b21b6', '#4c1d95'], emerald)      # violet
scale(['#fdf4ff', '#fae8ff', '#f5d0fe', '#f0abfc', '#e879f9', '#d946ef', '#c026d3', '#a21caf', '#86198f', '#701a75'], amber)        # fuchsia
scale(['#fdf2f8', '#fce7f3', '#fbcfe8', '#f9a8d4', '#f472b6', '#ec4899', '#db2777', '#be185d', '#9d174d', '#831843'], amber)        # pink
scale(['#eef2ff', '#e0e7ff', '#c7d2fe', '#a5b4fc', '#818cf8', '#6366f1', '#4f46e5', '#4338ca', '#3730a3', '#312e81'], teal)         # indigo
HEX.update({
    '#7b2ff7': '#059669', '#7c3aed': '#059669',
    # near-black purple backgrounds -> Axomai dark navy
    '#06060b': '#0b1220', '#0c0d16': '#0d1626', '#0e0f1c': '#0f1a2b', '#0e0c1f': '#0f1a2b', '#141228': '#111c2e',
    '#0d0b1a': '#0b1220', '#0b0b14': '#0b1220', '#0d0c18': '#0b1220',
})

# ---- rgb triples used inside rgba(...)
RGB = {
    (168, 85, 247): (16, 185, 129), (192, 132, 252): (52, 211, 153), (139, 92, 246): (16, 185, 129), (167, 139, 250): (52, 211, 153),
    (124, 58, 237): (5, 150, 105), (147, 51, 234): (5, 150, 105), (123, 47, 247): (5, 150, 105), (126, 34, 206): (4, 120, 87),
    (236, 72, 153): (245, 158, 11), (232, 121, 249): (251, 191, 36), (217, 70, 239): (245, 158, 11), (244, 114, 182): (251, 191, 36),
    (99, 102, 241): (20, 184, 166), (129, 140, 248): (45, 212, 191),
    (6, 6, 11): (11, 18, 32),
}
FONT_LINK_OLD = re.compile(r'https://fonts\.googleapis\.com/css2\?family=Inter[^"\']*')
FONT_LINK_NEW = ('https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:ital,wght@0,400;0,500;0,600;0,700;0,800;1,400;1,600'
                 '&family=Noto+Sans+Bengali:wght@400;600;700&display=swap')

stats = {'hex': 0, 'rgba': 0, 'font': 0, 'files': 0}
hex_re = re.compile(r'#[0-9a-fA-F]{6}\b')
rgba_re = re.compile(r'(rgba?\(\s*)(\d{1,3})(\s*,\s*)(\d{1,3})(\s*,\s*)(\d{1,3})')


def convert(text):
    def h(m):
        new = HEX.get(m.group(0).lower())
        if new:
            stats['hex'] += 1
            return new
        return m.group(0)

    def r(m):
        key = (int(m.group(2)), int(m.group(4)), int(m.group(6)))
        new = RGB.get(key)
        if new:
            stats['rgba'] += 1
            return '%s%d%s%d%s%d' % (m.group(1), new[0], m.group(3), new[1], m.group(5), new[2])
        return m.group(0)
    text = hex_re.sub(h, text)
    text = rgba_re.sub(r, text)
    if 'fonts.googleapis.com' in text:
        text, n = FONT_LINK_OLD.subn(FONT_LINK_NEW, text)
        stats['font'] += n
    out = []
    for line in text.split('\n'):
        low = line.lower()
        if 'font' in low:
            n0 = line
            line = re.sub(r"(?<![A-Za-z])Inter(?![A-Za-z])", 'Plus Jakarta Sans', line)
            line = line.replace('Instrument Serif', 'Plus Jakarta Sans').replace('Noto Serif Bengali', 'Noto Sans Bengali')
            if line != n0:
                stats['font'] += 1
        out.append(line)
    return '\n'.join(out)


for dirpath, dirs, files in os.walk(root):
    dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
    for f in files:
        if not f.endswith(EXT):
            continue
        p = os.path.join(dirpath, f)
        if f in ('next.config.mjs', 'package-lock.json'):
            continue
        s = open(p, encoding='utf-8').read()
        new = convert(s)
        if f == 'tailwind.config.js':
            continue  # handled below
        if new != s:
            open(p, 'w', encoding='utf-8').write(new)
            stats['files'] += 1

# ---- tailwind: the old colour families now mean the Axomai colours, and the fonts
tw = os.path.join(root, 'tailwind.config.js')
s = open(tw, encoding='utf-8').read()
if 'AXOMAI-REBRAND' not in s:
    s = s.replace("module.exports = {", "// AXOMAI-REBRAND: colours and font of the Axomai Browser site.\nconst colors = require('tailwindcss/colors');\n\nmodule.exports = {", 1)
    s = re.sub(r"colors: \{\s*brand: \{.*?\n        \}\n      \},", "colors: {\n        brand: colors.emerald,\n        purple: colors.emerald,\n        violet: colors.emerald,\n        fuchsia: colors.amber,\n        pink: colors.amber,\n        indigo: colors.teal,\n      },", s, count=1, flags=re.S)
    assert 'brand: colors.emerald' in s, 'colour block not replaced'
    s = s.replace('\'"Inter"\'', "'\"Plus Jakarta Sans\"'").replace('"Inter"', '"Plus Jakarta Sans"')
    s = s.replace('"Instrument Serif"', '"Plus Jakarta Sans"').replace('"Noto Serif Bengali"', '"Noto Sans Bengali"')
    open(tw, 'w', encoding='utf-8').write(s)
print('rebrand done:', stats)
