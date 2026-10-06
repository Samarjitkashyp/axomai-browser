# -*- coding: utf-8 -*-
"""The About page (English): who is behind Axomai Browser, the parent company Axom AI (aiaxom.co.in) and what the browser is.

`build()` takes the finished English landing page and swaps in the About content, title, description, canonical address
and structured data, so the header, footer, menu and tracking stay identical on every page.
"""
import html
import json
import re

PARENT_URL = 'https://aiaxom.co.in/'
TITLE = 'About Axomai Browser and Axom AI | Made in Assam'
DESC = ('Axomai Browser is a free Windows browser with a built-in AI assistant, a product of Axom AI (aiaxom.co.in), '
        "Assam's own artificial intelligence company based in Hajo, Guwahati.")

FAQ = [
    ('Who makes Axomai Browser?',
     "Axomai Browser is made by Samarjit Kashyap, lead AI researcher at Axom AI, with Axom AI's founder Debajit Malakar. Axom AI (aiaxom.co.in) is an Assam-based artificial intelligence company."),
    ('Is Axomai Browser part of Axom AI?',
     'Yes. Axom AI (https://aiaxom.co.in/) is the parent company. Axomai Browser is one of its products, and it brings the Axom AI assistant into everyday web browsing.'),
    ('What is Axom AI?',
     "Axom AI is an Assam-based artificial intelligence platform in Guwahati built for Assamese speakers. Its website lists an Assamese chatbot, document and PDF tools, image generation, file utilities, web search, data analysis and translation, in Assamese, English and Hindi."),
    ('Is Axomai Browser free and open source?',
     'Yes. Axomai Browser is free to download and use, needs no account, and its source code is published on GitHub under the MIT licence.'),
    ('Where can I download Axomai Browser?',
     'From https://axomai-browser.aiaxom.co.in/. It runs on Windows 10 and Windows 11 (64-bit).'),
]

CSS = """
.about{overflow-x:clip}
.ahero{position:relative;padding:34px 0 40px;background:radial-gradient(900px 380px at 12% -60px,var(--soft),transparent 70%),radial-gradient(700px 300px at 95% 10%,rgba(245,158,11,.14),transparent 70%);border-bottom:1px solid var(--line)}
.crumbs{font-size:.86rem;color:var(--muted);margin:0 0 18px}.crumbs a{color:var(--muted);text-decoration:none}.crumbs a:hover{color:var(--acc2)}
.ahero h1{max-width:20ch;margin:14px 0 16px;font-size:clamp(2.1rem,5.6vw,3.6rem);text-align:left}
.ahero .lead{max-width:62ch;margin:0 0 22px;text-align:left}
.chips{display:flex;flex-wrap:wrap;gap:10px;margin:0 0 26px;padding:0;list-style:none}
.chips li{background:var(--card);border:1px solid var(--line);border-radius:999px;padding:7px 14px;font-size:.86rem;font-weight:700;color:var(--text)}
.ahero .acts{display:flex;flex-wrap:wrap;gap:12px}
.about section{padding:44px 0}.about section.alt{background:color-mix(in srgb,var(--acc) 7%,var(--bg1));border-top:1px solid var(--line);border-bottom:1px solid var(--line)}
.shead{max-width:680px;margin:0 0 20px}.shead .kicker{margin-bottom:8px}
.about h2{font-size:clamp(1.55rem,3.6vw,2.2rem);line-height:1.2;margin:0 0 12px;color:var(--head);letter-spacing:-.01em}
.about p{max-width:70ch;margin:0 0 14px}
.hgrid{display:grid;grid-template-columns:1.25fr 1fr;gap:48px;align-items:center}
.family{display:grid;grid-template-columns:1fr;gap:10px;align-items:stretch;margin:0}
.fam{background:var(--card);border:1px solid var(--line);border-radius:20px;padding:22px 24px;box-shadow:var(--shadow)}
.fam small{display:block;font-size:.74rem;letter-spacing:.08em;text-transform:uppercase;color:var(--muted);margin-bottom:6px;font-weight:700}
.fam b{display:block;font-size:1.25rem;color:var(--head);margin-bottom:6px}.fam p{margin:0;font-size:.95rem}
.fam.main{background:linear-gradient(135deg,var(--acc2),#064e3b);border-color:transparent}.fam.main small,.fam.main b,.fam.main p{color:#fff}.fam.main small{opacity:.8}
.arrow{justify-self:center;font-size:1.6rem;line-height:1;color:var(--acc);font-weight:800;transform:rotate(90deg)}
.duo{display:grid;grid-template-columns:1fr 1fr;gap:48px;align-items:start}.duo .shead{margin-bottom:16px}
.about .band h2{color:#fff}.about .band p{margin:0 auto 20px;max-width:640px;text-align:center}
.about section.slim{padding:30px 0}
.parent{display:grid;grid-template-columns:1.25fr 1fr;gap:40px;align-items:start}
.pcard{background:var(--card);border:1px solid var(--line);border-radius:20px;padding:24px;box-shadow:var(--shadow)}
.pcard h3{margin:0 0 4px;font-size:1.1rem;color:var(--head)}.pcard dl{margin:14px 0 0;display:grid;gap:14px}
.pcard dt{font-size:.72rem;letter-spacing:.08em;text-transform:uppercase;color:var(--muted)}.pcard dd{margin:2px 0 0;font-weight:700;color:var(--head)}
.people{display:grid;grid-template-columns:repeat(2,1fr);gap:16px;margin:22px 0 0}
.person{display:flex;gap:14px;align-items:center;background:var(--card);border:1px solid var(--line);border-radius:16px;padding:16px}
.person i{flex:none;width:48px;height:48px;border-radius:50%;display:grid;place-items:center;font-style:normal;font-weight:800;color:#fff;background:linear-gradient(135deg,var(--acc),var(--acc2))}
.person b{display:block;color:var(--head)}.person span{font-size:.88rem;color:var(--muted)}
.fgrid2{display:grid;grid-template-columns:repeat(3,1fr);gap:16px}
.fcard{background:var(--card);border:1px solid var(--line);border-radius:18px;padding:22px;transition:transform .15s,box-shadow .15s}
.fcard:hover{transform:translateY(-3px);box-shadow:var(--shadow)}
.fcard span{display:grid;place-items:center;width:44px;height:44px;border-radius:12px;background:var(--bg2);font-size:1.4rem;margin-bottom:12px}
.fcard b{display:block;color:var(--head);margin-bottom:6px;font-size:1.05rem}.fcard p{margin:0;font-size:.94rem;color:var(--text)}
.why{display:grid;grid-template-columns:1.1fr 1fr;gap:34px;align-items:center}
.quote{background:var(--card);border:1px solid var(--line);border-left:5px solid var(--acc);border-radius:16px;padding:22px 24px;font-size:1.05rem;color:var(--head);font-weight:600}
.about table.facts{max-width:none;margin:0}
.about details{max-width:none}
@media (max-width:980px){.fgrid2{grid-template-columns:repeat(2,1fr)}}
@media (max-width:980px){.hgrid,.duo{grid-template-columns:1fr;gap:28px}}
@media (max-width:860px){.ahero{padding:28px 0 30px}.about section{padding:34px 0}.parent,.why{grid-template-columns:1fr;gap:22px}.people{grid-template-columns:1fr}.ahero .acts .btn{width:100%}}
@media (max-width:560px){.fgrid2{grid-template-columns:1fr}.chips li{font-size:.8rem;padding:6px 11px}}
"""


def body(ctx, dl):
    faq = ''.join('<details%s><summary>%s</summary><p>%s</p></details>' % (' open' if i == 0 else '', html.escape(q), html.escape(a)) for i, (q, a) in enumerate(FAQ))
    feats = [('🤖', 'Axom AI built in', 'Ask anything, or ask about the page you are reading. Answers stream in and chats are not saved.'),
             ('🗣️', 'Speaks your language', 'Menus and settings in Assamese, Hindi, Bengali and English, plus page translation and read-aloud.'),
             ('🌿', 'Made for Assam', 'Headlines from Assamese and North-East news sources, weather for 35 towns and five Assam-inspired themes.'),
             ('🛡️', 'Private by default', 'Ad and tracker blocking, HTTPS-only mode, per-site permissions and no account. The AI key stays on our server.'),
             ('🧰', 'Everyday tools', 'Tab groups and search, split view, bookmarks, downloads, Chrome extensions (load unpacked) and encrypted folder sync.'),
             ('🔓', 'Free and open', 'No price and no sign-in. Source code on GitHub under the MIT licence; updates are checked against a SHA-256 checksum.')]
    fc = ''.join('<div class="fcard"><span aria-hidden="true">%s</span><b>%s</b><p>%s</p></div>' % (e, html.escape(t), html.escape(d)) for e, t, d in feats)
    return f'''<main id="main" class="about">
<div class="ahero"><div class="wrap"><div class="hgrid"><div>
<p class="crumbs"><a href="/">Axomai Browser</a> &nbsp;/&nbsp; About</p>
<span class="eyebrow">ABOUT US</span>
<h1>Built in Assam by <em>Axom AI</em></h1>
<p class="lead">Axomai Browser is a free Windows web browser with a built-in AI assistant. It is a product of <a href="{PARENT_URL}" rel="noopener">Axom AI</a> (aiaxom.co.in), an artificial intelligence company from Assam, India.</p>
<ul class="chips"><li>Free</li><li>Windows 10 &amp; 11</li><li>Open source</li><li>Assamese · Hindi · Bengali · English</li></ul>
<div class="acts"><a class="btn" href="{dl}" data-dl>Download for Windows</a><a class="btn ghost" href="{PARENT_URL}" rel="noopener">Visit aiaxom.co.in</a></div>
</div>
<div class="family" aria-label="How Axom AI and Axomai Browser are related">
<div class="fam"><small>Parent company</small><b>Axom AI</b><p>Assam's own artificial intelligence platform, at aiaxom.co.in.</p></div>
<div class="arrow" aria-hidden="true">&rarr;</div>
<div class="fam main"><small>Product</small><b>Axomai Browser</b><p>The Axom AI assistant, inside a free Windows browser.</p></div>
</div></div></div></div>

<section id="parent" aria-labelledby="parent-h"><div class="wrap">
<div class="shead"><div class="kicker">THE COMPANY</div><h2 id="parent-h">The company behind it: Axom AI</h2></div>
<div class="parent">
<div>
<p><a href="{PARENT_URL}" rel="noopener">Axom AI</a> is Assam's own artificial intelligence platform, built in Guwahati for people who speak Assamese. Its goal is simple: AI that understands Assamese language and culture, instead of treating Assamese as an afterthought.</p>
<p>According to its website, Axom AI offers an Assamese chatbot, document and PDF reading, image generation, more than twenty file tools, live web search, spreadsheet analysis and translation. It works in Assamese, English and Hindi, and has a free plan.</p>
<div class="people">
<div class="person"><i aria-hidden="true">DM</i><div><b>Debajit Malakar</b><span>Founder, Axom AI</span></div></div>
<div class="person"><i aria-hidden="true">SK</i><div><b>Samarjit Kashyap</b><span>Lead AI researcher, author of Axomai Browser</span></div></div>
</div>
</div>
<aside class="pcard" aria-label="Axom AI at a glance"><h3>Axom AI at a glance</h3>
<dl><div><dt>Website</dt><dd><a href="{PARENT_URL}" rel="noopener">aiaxom.co.in</a></dd></div>
<div><dt>Based in</dt><dd>Hajo, Guwahati, Assam, India</dd></div>
<div><dt>Languages</dt><dd>Assamese, English, Hindi</dd></div>
<div><dt>Product from Axom AI</dt><dd>Axomai Browser</dd></div></dl></aside>
</div>
</div></section>

<section id="browser" class="alt" aria-labelledby="browser-h"><div class="wrap">
<div class="shead"><div class="kicker">THE PRODUCT</div><h2 id="browser-h">What Axomai Browser is</h2>
<p>A desktop browser for Windows 10 and 11. Web pages run on the Microsoft Edge WebView2 (Chromium) engine, so websites work the way they do in other modern browsers. The tabs, toolbar, settings and built-in pages are Axomai's own.</p></div>
<div class="fgrid2">{fc}</div>
</div></section>

<section id="why" class="slim" aria-labelledby="why-h"><div class="wrap"><div class="why">
<div><div class="shead" style="margin:0"><div class="kicker">THE IDEA</div><h2 id="why-h">Why Axom AI made a browser</h2>
<p>Most people meet AI inside a browser tab. Putting Axom AI inside the browser means a student in Jorhat or a shopkeeper in Guwahati can ask a question in Assamese about the page in front of them, without copying text into another app.</p></div></div>
<div class="quote">Axomai Browser is how Axom AI reaches everyday browsing: in your language, private by default, and free.</div>
</div></div></section>

<section id="facts" class="alt" aria-labelledby="facts-h"><div class="wrap"><div class="duo">
<div><div class="shead"><div class="kicker">AT A GLANCE</div><h2 id="facts-h">Quick facts</h2></div>
<table class="facts"><tbody>
<tr><th scope="row">Product</th><td>Axomai Browser (also written Axom AI Browser)</td></tr>
<tr><th scope="row">Parent company</th><td><a href="{PARENT_URL}" rel="noopener">Axom AI (aiaxom.co.in)</a></td></tr>
<tr><th scope="row">Author</th><td>Samarjit Kashyap, lead AI researcher at Axom AI</td></tr>
<tr><th scope="row">Platform</th><td>Windows 10 and 11 (64-bit)</td></tr>
<tr><th scope="row">Version</th><td>{ctx['version']}</td></tr>
<tr><th scope="row">Price</th><td>Free</td></tr>
<tr><th scope="row">Licence</th><td>MIT, open source</td></tr>
<tr><th scope="row">Interface languages</th><td>Assamese, Hindi, Bengali, English</td></tr>
</tbody></table>
</div>

<div id="faq"><div class="shead"><div class="kicker">FAQ</div><h2 id="faq-h">Questions about us</h2></div>
{faq}
</div>
</div></div></section>

<section class="slim"><div class="wrap"><div class="band"><h2>Try Axomai Browser</h2><p>Free for Windows 10 and 11. No account needed.</p><a class="btn" href="{dl}" data-dl>Download for Windows</a></div></div></section>
</main>'''


def graph(site, ctx, github):
    about_url = site + '/about/'
    parent = {'@type': 'Organization', '@id': PARENT_URL + '#org', 'name': 'Axom AI', 'alternateName': ['AIAXOM', 'Axom AI Assam'], 'url': PARENT_URL,
              'description': "Assam's own artificial intelligence platform for Assamese speakers, based in Guwahati.",
              'address': {'@type': 'PostalAddress', 'addressLocality': 'Hajo, Guwahati', 'postalCode': '781102', 'addressRegion': 'Assam', 'addressCountry': 'IN'},
              'founder': {'@type': 'Person', 'name': 'Debajit Malakar'},
              'employee': {'@type': 'Person', 'name': 'Samarjit Kashyap', 'jobTitle': 'Lead AI Researcher'},
              'knowsLanguage': ['as', 'en', 'hi'], 'brand': {'@id': site + '/#org'}}
    return [
        parent,
        {'@type': 'Organization', '@id': site + '/#org', 'name': 'Axomai', 'url': site + '/', 'parentOrganization': {'@id': PARENT_URL + '#org'},
         'logo': {'@type': 'ImageObject', 'url': site + '/assets/logo-512.png', 'width': 512, 'height': 512}, 'sameAs': [github, PARENT_URL]},
        {'@type': 'AboutPage', '@id': about_url + '#webpage', 'url': about_url, 'name': TITLE, 'description': DESC, 'inLanguage': 'en-IN',
         'isPartOf': {'@id': site + '/#website'}, 'mainEntity': {'@id': PARENT_URL + '#org'}, 'dateModified': ctx['date'],
         'primaryImageOfPage': {'@type': 'ImageObject', 'url': site + '/assets/og-image.jpg'}},
        {'@type': 'WebSite', '@id': site + '/#website', 'url': site + '/', 'name': 'Axomai Browser', 'publisher': {'@id': site + '/#org'}},
        {'@type': 'SoftwareApplication', '@id': site + '/#app', 'name': 'Axomai Browser', 'applicationCategory': 'BrowserApplication', 'operatingSystem': 'Windows 10, Windows 11',
         'url': site + '/', 'author': {'@id': site + '/#org'}, 'publisher': {'@id': PARENT_URL + '#org'}, 'isAccessibleForFree': True,
         'offers': {'@type': 'Offer', 'price': '0', 'priceCurrency': 'INR'}},
        {'@type': 'FAQPage', '@id': about_url + '#faq', 'mainEntity': [{'@type': 'Question', 'name': q, 'acceptedAnswer': {'@type': 'Answer', 'text': a}} for q, a in FAQ]},
        {'@type': 'BreadcrumbList', 'itemListElement': [{'@type': 'ListItem', 'position': 1, 'name': 'Axomai Browser', 'item': site + '/'},
                                                         {'@type': 'ListItem', 'position': 2, 'name': 'About', 'item': about_url}]},
    ]


def build(home_html, site, ctx, github, dl):
    """Turns the rendered English landing page into the About page."""
    about_url = site + '/about/'
    h = home_html
    e = html.escape

    def sub(pattern, repl, flags=0):
        nonlocal h
        new, n = re.subn(pattern, lambda m: repl, h, count=1, flags=flags)
        assert n == 1, pattern
        h = new

    sub(r'<title>.*?</title>', '<title>%s</title>' % e(TITLE))
    sub(r'<meta name="description" content="[^"]*">', '<meta name="description" content="%s">' % e(DESC))
    sub(r'<link rel="canonical" href="[^"]*">', '<link rel="canonical" href="%s">' % about_url)
    h = re.sub(r'<link rel="alternate" hreflang="[^"]*" href="[^"]*">', '', h)
    sub(r'<meta property="og:title" content="[^"]*">', '<meta property="og:title" content="%s">' % e(TITLE))
    sub(r'<meta property="og:description" content="[^"]*">', '<meta property="og:description" content="%s">' % e(DESC))
    sub(r'<meta property="og:url" content="[^"]*">', '<meta property="og:url" content="%s">' % about_url)
    sub(r'<meta name="twitter:title" content="[^"]*">', '<meta name="twitter:title" content="%s">' % e(TITLE))
    sub(r'<meta name="twitter:description" content="[^"]*">', '<meta name="twitter:description" content="%s">' % e(DESC))
    ld = json.dumps({'@context': 'https://schema.org', '@graph': graph(site, ctx, github)}, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
    sub(r'<script type="application/ld\+json">.*?</script>', '<script type="application/ld+json">%s</script>' % ld, re.S)
    sub(r'</style>', CSS + '</style>')
    sub(r'<main id="main">.*?</main>', body(ctx, dl), re.S)
    # the header, menu and footer links point at sections of the home page
    h = h.replace('<a href="/about/">About</a>', '<a href="/about/" aria-current="page">About</a>')
    h = h.replace('href="#', 'href="/#')
    h = h.replace('href="/#main"', 'href="#main"')
    return h
