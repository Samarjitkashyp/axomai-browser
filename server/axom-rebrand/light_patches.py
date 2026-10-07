# -*- coding: utf-8 -*-
"""The hand-written part of the light theme (run after lighten.py, on the same copy of next-frontend):
the theme switch in <head>, the pages that must stay dark, the Navbar logo and drawer, the dark footer and the light CSS."""
import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else '.'


def path(*p):
    return os.path.join(root, *p)


def read(p):
    return open(path(p), encoding='utf-8').read()


def write(p, s):
    open(path(p), 'w', encoding='utf-8').write(s)


def rep(s, old, new, count=1):
    assert old in s, 'not found: ' + old[:80]
    return s.replace(old, new, count)


# ---------------------------------------------------------------- layout.tsx: follow the OS (light or dark), tools and chat stay dark
s = read('app/layout.tsx')
if 'AXOMAI-LIGHT' not in s:
    s = rep(s, "const GTM_ID = 'GTM-K4N88ZBR';",
            "const GTM_ID = 'GTM-K4N88ZBR';\n\n"
            "// AXOMAI-LIGHT: the site follows the OS theme (light is the Axomai Browser look). The tools and the chat keep their dark look.\n"
            "const THEME_JS = `(function(){try{var h=document.documentElement,q=window.matchMedia('(prefers-color-scheme: dark)');"
            "function app(){return /^\\\\/(tools|chat)(\\\\/|$)/.test(location.pathname)}"
            "h.classList.toggle('dark',app()||q.matches);"
            "q.addEventListener('change',function(e){if(!app())h.classList.toggle('dark',e.matches)})}catch(e){}})();`;")
    s = re.sub(r"themeColor: '#[0-9a-fA-F]{6}',",
               "themeColor: [\n    { media: '(prefers-color-scheme: light)', color: '#f0fdf4' },\n    { media: '(prefers-color-scheme: dark)', color: '#0b1220' },\n  ],", s, count=1)
    s = rep(s, '<html lang="en-IN" className="scroll-smooth">', '<html lang="en-IN" className="scroll-smooth" suppressHydrationWarning>')
    s = rep(s, "      <head>\n", "      <head>\n        <script dangerouslySetInnerHTML={{ __html: THEME_JS }} />\n")
    write('app/layout.tsx', s)

# ---------------------------------------------------------------- ForceDark for /tools and /chat
write('app/ForceDark.tsx', """'use client';

import { useEffect } from 'react';

// The tools and the chat have their own dark look: keep the `dark` class on <html> while they are on screen.
export default function ForceDark() {
  useEffect(() => {
    const el = document.documentElement;
    const had = el.classList.contains('dark');
    el.classList.add('dark');
    return () => {
      if (!had && !window.matchMedia('(prefers-color-scheme: dark)').matches) el.classList.remove('dark');
    };
  }, []);
  return null;
}
""")
if not os.path.exists(path('app/tools/layout.tsx')):
    write('app/tools/layout.tsx', """import ForceDark from '../ForceDark';

export default function ToolsLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <ForceDark />
      {children}
    </>
  );
}
""")
s = read('app/chat/layout.tsx')
if 'ForceDark' not in s:
    s = rep(s, "import './chat.css';", "import './chat.css';\nimport ForceDark from '../ForceDark';")
    s = rep(s, "  return <>{children}</>;", "  return (\n    <>\n      <ForceDark />\n      {children}\n    </>\n  );")
    write('app/chat/layout.tsx', s)

# ---------------------------------------------------------------- Navbar: logo for light pages, the drawer stays dark
s = read('components/Navbar.tsx')
s = s.replace('className="w-auto h-auto transition-transform duration-300 group-hover:scale-105"', 'className="axom-logo w-auto h-auto transition-transform duration-300 group-hover:scale-105"') if 'axom-logo w-auto' not in s else s
s = s.replace("className={`drawer lg:hidden ${open ? 'open' : ''}`}", "className={`dark drawer lg:hidden ${open ? 'open' : ''}`}")
assert 'axom-logo w-auto' in s and 'dark drawer' in s
write('components/Navbar.tsx', s)

# ---------------------------------------------------------------- the footer is dark in both themes (like the Axomai Browser site)
s = read('components/Footer.tsx')
s = s.replace('bg-black/80 mt-20 relative', 'bg-[#07111f] mt-20 relative')
write('components/Footer.tsx', s)

# ---------------------------------------------------------------- home page: the hero photo and the last banner keep their dark look
s = read('app/page.tsx')
if 'AXOMAI-LIGHT' not in s:
    s = rep(s, '        <section className="relative min-h-[88vh]', '        {/* AXOMAI-LIGHT: the hero photo stays dark in both themes */}\n        <div className="dark">\n        <section className="relative min-h-[88vh]')
    s = rep(s, '        </section>\n\n        {/* ==================== LOGO STRIP', '        </section>\n        </div>\n\n        {/* ==================== LOGO STRIP')
    s = rep(s, '        <section className="assam-bg py-24', '        <div className="dark">\n        <section className="assam-bg py-24')
    s = rep(s, '        </section>\n\n      </main>', '        </section>\n        </div>\n\n      </main>')
    write('app/page.tsx', s)

# ---------------------------------------------------------------- globals.css: the light theme
css = read('app/globals.css')
if 'AXOMAI-LIGHT' not in css:
    css += """

/* =========================================================
   AXOMAI-LIGHT: light theme (the Axomai Browser look).
   Tailwind classes carry their own `dark:` variants; this block restyles the hand-written classes.
   Anything inside a `.dark` element (the hero photo, the drawer) keeps the dark look.
========================================================= */
html:not(.dark) { --bg-dark: #f0fdf4; }
html:not(.dark) body { background: #f0fdf4; color: #064e3b; }
html:not(.dark) ::-webkit-scrollbar-track { background: #f0fdf4; }
html:not(.dark) ::-webkit-scrollbar-thumb { border-color: #f0fdf4; }
html:not(.dark) * { scrollbar-color: #10b981 #f0fdf4; }

/* the navbar logo has white lettering: make it dark on the light bar */
html:not(.dark) .axom-logo:not(:is(.dark *)) { filter: invert(1) hue-rotate(180deg); }

/* the closed mobile drawer must not cast its shadow onto the page */
.drawer:not(.open) { box-shadow: none !important; }

html:not(.dark) .grid-bg:not(:is(.dark *)) {
  background-image:
    linear-gradient(rgba(5,150,105,0.07) 1px, transparent 1px),
    linear-gradient(90deg, rgba(5,150,105,0.07) 1px, transparent 1px);
}
html:not(.dark) .gradient-text:not(:is(.dark *)) { background-image: linear-gradient(120deg, #059669, #d97706, #059669); }
html:not(.dark) .btn-ghost:not(:is(.dark *)) { background: rgba(6,78,59,0.05); border-color: rgba(6,78,59,0.2); color: #064e3b; }
html:not(.dark) .btn-ghost:not(:is(.dark *)):hover { background: rgba(6,78,59,0.1); border-color: rgba(6,78,59,0.35); }
html:not(.dark) .glass-card:not(:is(.dark *)) {
  background: rgba(255,255,255,0.88);
  border: 1px solid #cfe9dc;
  box-shadow: 0 10px 30px -14px rgba(5,150,105,0.22);
}
html:not(.dark) .glass-card:not(:is(.dark *)):hover { border-color: rgba(5,150,105,0.5); box-shadow: 0 20px 45px -18px rgba(5,150,105,0.3); }
html:not(.dark) .featured-card:not(:is(.dark *)) {
  background: linear-gradient(135deg, #ffffff 0%, #ecfdf5 100%);
  border: 1px solid rgba(16,185,129,0.45);
  box-shadow: 0 20px 50px -20px rgba(5,150,105,0.3);
}
html:not(.dark) .hero-assam-bg:not(:is(.dark *)) {
  background: radial-gradient(circle at 50% 0%, rgba(16,185,129,0.22) 0%, rgba(245,158,11,0.1) 40%, rgba(240,253,244,0) 75%), linear-gradient(180deg, #f0fdf4 0%, #ecfdf5 100%);
}
html:not(.dark) .assam-bg:not(:is(.dark *)) {
  background-image: linear-gradient(180deg, rgba(240,253,244,0.93) 0%, rgba(236,253,245,0.9) 100%), url("https://aiaxom.co.in/static/dist/hero/assam.avif");
}
html:not(.dark) .mega-item:hover { background: rgba(16,185,129,0.1); }
html:not(.dark) .mega-icon { background: linear-gradient(135deg, rgba(16,185,129,0.15), rgba(245,158,11,0.12)); border-color: rgba(16,185,129,0.3); }
html:not(.dark) .cat-btn.active {
  background: linear-gradient(135deg, rgba(245,158,11,0.2) 0%, rgba(16,185,129,0.2) 100%);
  border-color: rgba(217,119,6,0.6);
  color: #92400e;
  box-shadow: 0 0 15px rgba(245,158,11,0.2);
}

/* blog articles */
html:not(.dark) .blog-rich-content { color: #334155; }
html:not(.dark) .blog-rich-content h1, html:not(.dark) .blog-rich-content h4 { color: #022c22; }
html:not(.dark) .blog-rich-content h2 { color: #b45309; }
html:not(.dark) .blog-rich-content h3 { color: #047857; }
html:not(.dark) .blog-rich-content strong, html:not(.dark) .blog-rich-content b { color: #022c22; }
html:not(.dark) .blog-rich-content li { color: #334155; }
html:not(.dark) .blog-rich-content a:not([class*="btn"]):not([class*="bg-"]):not([class*="rounded"]) { color: #047857; }
html:not(.dark) .blog-rich-content a:not([class*="btn"]):not([class*="bg-"]):not([class*="rounded"]):hover { color: #b45309; }
html:not(.dark) .blog-rich-content img { border-color: rgba(6,78,59,0.15); }
html:not(.dark) .blog-rich-content blockquote {
  background: linear-gradient(90deg, rgba(16,185,129,0.12) 0%, rgba(16,185,129,0.02) 100%);
  color: #92400e;
  border-top-color: rgba(6,78,59,0.08); border-bottom-color: rgba(6,78,59,0.08); border-right-color: rgba(6,78,59,0.08);
}
html:not(.dark) .blog-rich-content code { background: rgba(6,78,59,0.07); color: #b45309; border-color: rgba(6,78,59,0.12); }
html:not(.dark) .blog-rich-content pre code { background: transparent; border-color: transparent; color: inherit; }
html:not(.dark) .blog-rich-content table, html:not(.dark) .blog-rich-content .table-wrapper { border-color: #cfe9dc; }
html:not(.dark) .blog-rich-content th, html:not(.dark) .blog-rich-content td { border-color: #cfe9dc; }
html:not(.dark) .blog-rich-content th { background: #d1fae5; color: #022c22; }
html:not(.dark) .blog-rich-content tr:nth-child(even) { background: rgba(6,78,59,0.03); }
html:not(.dark) .blog-rich-content .comparison-table td, html:not(.dark) .blog-rich-content .ai-table td { color: #334155; border-bottom-color: #d9ebe2; }
html:not(.dark) .blog-rich-content .comparison-table tr:nth-child(even) td, html:not(.dark) .blog-rich-content .ai-table tr:nth-child(even) td { background: rgba(6,78,59,0.03); }
html:not(.dark) .blog-rich-content .comparison-table th, html:not(.dark) .blog-rich-content .ai-table th { background: #0f172a; color: #ffffff; }
html:not(.dark) .blog-rich-content .highlight-box p, html:not(.dark) .blog-rich-content .quick-answer p { color: #334155; }
html:not(.dark) .blog-rich-content .faq-item, html:not(.dark) .blog-rich-content .faq-item:first-of-type { border-color: #cfe9dc; }
html:not(.dark) .blog-rich-content .faq-item h3 { color: #b45309; }
html:not(.dark) .blog-rich-content .faq-item p { color: #475569; }
/* the dark call-out boxes keep their own dark look and light text */
html:not(.dark) .blog-rich-content :is(.tool-card, .innovation-card, .feature-card, .action-box, .workflow, .workflow-box, .final-thoughts, .final-box, .blog-cta-box, .cta-box) { color: #cbd5e1; }
html:not(.dark) .blog-rich-content :is(.tool-card, .innovation-card, .feature-card, .final-thoughts, .final-box, .blog-cta-box, .cta-box) :is(h1, h2, h3, h4, strong, b) { color: #ffffff; }
html:not(.dark) .blog-rich-content :is(.action-box) h3 { color: #fde68a; }
html:not(.dark) .blog-rich-content :is(.workflow, .workflow-box) { color: #ffffff; }
html:not(.dark) .blog-rich-content :is(.tool-label, .feature-label, .card-number) { color: #fde68a; }
html:not(.dark) .blog-rich-content :is(.final-thoughts, .final-box, .blog-cta-box, .cta-box) p { color: #e2e8f0; }
html:not(.dark) .blog-rich-content :is(.action-box) p { color: #cbd5e1; }
"""
    write('app/globals.css', css)
print('light patches applied')
