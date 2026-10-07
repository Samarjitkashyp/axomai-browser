# -*- coding: utf-8 -*-
"""Adds a LIGHT theme to the Axom AI marketing pages (the Axomai Browser look: mint and white, emerald and amber).

Run it on a COPY of next-frontend, after rebrand.py:
    python3 lighten.py /path/to/copy-of/next-frontend

Every colour class that only makes sense on a dark page is rewritten as "<light class> dark:<old class>", so
  * with the OS in light mode (or no `dark` class on <html>) the page is light, and
  * with the `dark` class on <html> everything is exactly as it was before.
It works on the pages and components of the marketing site only (home, pricing, about, blog, faq, contact, use cases,
privacy, terms, Navbar). The footer stays dark in both themes, like the Axomai Browser site, and the tools and chat
keep their own dark look (they force the `dark` class).
"""
import colorsys
import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else '.'

PAGES = ['app/page.tsx', 'app/pricing/page.tsx', 'app/about/page.tsx', 'app/contact/page.tsx', 'app/use-cases/page.tsx',
         'app/faq/page.tsx', 'app/blog/page.tsx', 'app/blog/[slug]/page.tsx', 'app/privacy/page.tsx', 'app/terms/page.tsx']
COMPONENTS = ['Navbar.tsx', 'AboutFaq.tsx', 'BlogSearchFilter.tsx', 'ContactFaqInteractive.tsx', 'ContactFormInteractive.tsx',
              'FAQPageContent.tsx', 'FAQSection.tsx', 'PricingFaq.tsx', 'PricingPlansInteractive.tsx', 'PricingSection.tsx',
              'PrivacyInteractiveContent.tsx', 'TermsInteractiveContent.tsx', 'UseCasesFaq.tsx', 'UseCasesSection.tsx',
              'aboutFaqsData.ts', 'contactData.ts', 'faqFullData.ts', 'homeFaqsData.ts', 'pricingData.ts', 'privacyData.ts',
              'termsData.ts', 'useCasesData.ts']
FILES = PAGES + ['components/' + c for c in COMPONENTS]

NEUTRAL = ('slate', 'gray', 'zinc', 'neutral', 'stone')
ACCENT = ('emerald', 'teal', 'amber', 'orange', 'purple', 'fuchsia', 'pink', 'violet', 'indigo', 'blue', 'cyan', 'sky', 'rose', 'red', 'green', 'lime', 'yellow')
FAMILY = '|'.join(NEUTRAL + ACCENT)

TOKEN = re.compile(
    r'(?<=[\s"\'`{(,])((?:[a-z0-9\-\[\]]+:)*)(text|bg|border|from|via|to|divide)-'
    r'(white|black|(?:%s)-\d{2,3}|\[#[0-9a-fA-F]{3,8}\])(?:/(\d{1,3}))?(?![A-Za-z0-9_\-/\]\[])' % FAMILY)
SOLID_BG = re.compile(r'(?<![:\w-])bg-(?:gradient-to-\w+|(?:%s)-[4-9]00(?!/))(?![\w/])' % '|'.join(ACCENT))
STRING = re.compile(r'"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\'|`(?:\\.|[^`\\])*`', re.S)

stats = {'tokens': 0, 'strings': 0, 'files': 0}
unmapped = {}


def lum(hexv):
    h = hexv.lstrip('#')
    if len(h) == 3:
        h = ''.join(c * 2 for c in h)
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    return colorsys.rgb_to_hls(r, g, b)[1]


def a(alpha):
    return '/' + alpha if alpha else ''


def border_alpha(n):
    return '10' if n <= 5 else '15' if n <= 10 else '20' if n <= 15 else '25'


def light(prefix, util, col, alpha, ctx):
    """The light-theme class for one dark-theme token, or None to leave it alone."""
    hov = any(x in prefix for x in ('hover', 'focus', 'group-hover', 'active'))
    n = int(alpha) if alpha else None
    fam, _, shade = col.partition('-')
    shade = int(shade) if shade.isdigit() else None
    is_hex = col.startswith('[#')

    if util == 'text':
        if col == 'white':
            return None if ctx['solid'] else 'text-emerald-950' + a(alpha)
        if is_hex:
            return 'text-emerald-950' if lum(col[1:-1]) > 0.8 else None
        if fam in NEUTRAL and shade is not None:
            if shade <= 200:
                return 'text-slate-800' + a(alpha)
            if shade == 300:
                return 'text-slate-700' + a(alpha)
            if shade in (400, 500):
                return 'text-slate-600' + a(alpha)
            return None
        if fam in ACCENT and shade in (200, 300, 400):
            return 'text-%s-%d%s' % (fam, 800 if shade == 200 else 700, a(alpha))
        return None

    if util == 'bg':
        if col == 'white':
            if n is None:
                return None
            if hov:
                return 'bg-emerald-50'
            if n <= 8:
                return 'bg-white/70'
            if n <= 14:
                return 'bg-emerald-900/5'
            return 'bg-emerald-900/10' if n <= 30 else None
        if col == 'black':
            if ctx['blur'] and n is not None and n >= 50 and not hov:
                return 'bg-white/%d' % max(n, 80)
            return None if (hov or n is None or n > 50) else 'bg-emerald-900/5'
        if is_hex:
            v = lum(col[1:-1])
            if v > 0.3:
                return None
            hx = col[2:-1].lower()
            return 'bg-[#%s]%s' % ('f0fdf4' if (ctx['screen'] or hx in ('0b1220', '0f1a2b', '0d1626') or v > 0.025) else 'ffffff', a(alpha))
        if fam in NEUTRAL and shade is not None and shade >= 700:
            if n is None:
                return 'bg-white'
            return 'bg-white/%d' % max(n, 70)
        if fam in ACCENT and shade in (900, 950) and n is not None:
            return 'bg-%s-50/%s' % (fam, alpha)
        return None

    if util in ('border', 'divide'):
        if col == 'white' and n is not None:
            return '%s-emerald-900/%s' % (util, border_alpha(n))
        if fam in NEUTRAL and shade is not None:
            if shade in (700, 800):
                return '%s-slate-200' % util
            if shade in (900, 950):
                return '%s-slate-200' % util
        return None

    # gradient stops
    if util in ('from', 'via', 'to'):
        if ctx['textgrad']:
            if col == 'white':
                return '%s-emerald-950%s' % (util, a(alpha))
            if fam in NEUTRAL and shade is not None and shade <= 400:
                return '%s-slate-700%s' % (util, a(alpha))
            if fam in ACCENT and shade in (200, 300, 400):
                return '%s-%s-%d%s' % (util, fam, 600 if shade != 200 else 500, a(alpha))
            return None
        if col == 'black':
            return '%s-emerald-50%s' % (util, a(alpha))
        if is_hex:
            v = lum(col[1:-1])
            return None if v > 0.3 else '%s-[#f0fdf4]%s' % (util, a(alpha))
        if fam in NEUTRAL and shade is not None and shade >= 800:
            return '%s-%s%s' % (util, 'white' if shade == 900 else 'emerald-50', a(alpha))
        if fam in ACCENT and shade in (800, 900, 950):
            return '%s-%s-100%s' % (util, fam, a(alpha))
        return None
    return None


def convert_string(s):
    ctx = {'solid': bool(SOLID_BG.search(s)), 'textgrad': 'bg-clip-text' in s, 'screen': 'min-h-screen' in s, 'blur': 'backdrop-blur' in s}
    changed = [0]

    def rep(m):
        prefix, util, col, alpha = m.group(1), m.group(2), m.group(3), m.group(4)
        new = light(prefix, util, col, alpha, ctx)
        old = '%s%s-%s%s' % (prefix, util, col, a(alpha))
        if new is None:
            if col in ('white',) or (col.startswith(('slate-', 'gray-')) and util in ('text', 'bg')):
                unmapped[old] = unmapped.get(old, 0) + 1
            return m.group(0)
        changed[0] += 1
        return '%s%s dark:%s' % (prefix, new, old)
    out = TOKEN.sub(rep, s)
    stats['tokens'] += changed[0]
    return out, changed[0]


def convert_file(text):
    def rep(m):
        s = m.group(0)
        if not TOKEN.search(s):
            return s
        out, n = convert_string(s)
        if n:
            stats['strings'] += 1
        return out
    return STRING.sub(rep, text)


for rel in FILES:
    p = os.path.join(root, rel)
    if not os.path.isfile(p):
        print('missing', rel)
        continue
    s = open(p, encoding='utf-8').read()
    new = convert_file(s)
    if new != s:
        open(p, 'w', encoding='utf-8').write(new)
        stats['files'] += 1
print('lighten done:', stats)
print('left as they are (dark tokens with no light rule):')
for k, v in sorted(unmapped.items(), key=lambda kv: -kv[1])[:25]:
    print('  %4d  %s' % (v, k))
