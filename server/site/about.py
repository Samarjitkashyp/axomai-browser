# -*- coding: utf-8 -*-
"""The About page (English): who is behind Axomai Browser, the parent company Axom AI (aiaxom.co.in) and what the browser is.

`ABOUT` holds every piece of text on the page (the admin panel edits it). `build()` takes the finished English landing page
and swaps in the About content, title, description, canonical address and structured data, so the header, footer, menu and
tracking stay identical on every page.

In text fields, `{parent}` becomes a link to Axom AI and `{version}` the current browser version.
"""
import html
import json
import re

PARENT_URL = 'https://aiaxom.co.in/'

ABOUT = dict(
    title='About Axomai Browser and Axom AI | Made in Assam',
    desc=('Axomai Browser is a free Windows browser with a built-in AI assistant, a product of Axom AI (aiaxom.co.in), '
          "Assam's own artificial intelligence company based in Hajo, Guwahati."),
    crumb='About',
    eyebrow='ABOUT US',
    h1_pre='Built in Assam by ',
    h1_em='Axom AI',
    lead='Axomai Browser is a free Windows web browser with a built-in AI assistant. It is a product of {parent} (aiaxom.co.in), an artificial intelligence company from Assam, India.',
    chips=['Free', 'Windows 10 & 11', 'Open source', 'Assamese · Hindi · Bengali · English'],
    btn_download='Download for Windows',
    btn_visit='Visit aiaxom.co.in',
    fam_parent_label='Parent company',
    fam_parent_p="Assam's own artificial intelligence platform, at aiaxom.co.in.",
    fam_product_label='Product',
    fam_product_p='The Axom AI assistant, inside a free Windows browser.',
    company_kicker='THE COMPANY',
    company_h='The company behind it: Axom AI',
    company_p1="{parent} is Assam's own artificial intelligence platform, built in Guwahati for people who speak Assamese. Its goal is simple: AI that understands Assamese language and culture, instead of treating Assamese as an afterthought.",
    company_p2='According to its website, Axom AI offers an Assamese chatbot, document and PDF reading, image generation, more than twenty file tools, live web search, spreadsheet analysis and translation. It works in Assamese, English and Hindi, and has a free plan.',
    people=[['DM', 'Debajit Malakar', 'Founder, Axom AI'], ['SK', 'Samarjit Kashyap', 'Lead AI researcher, author of Axomai Browser']],
    glance_h='Axom AI at a glance',
    glance=[['Website', 'aiaxom.co.in'], ['Based in', 'Hajo, Guwahati, Assam, India'], ['Languages', 'Assamese, English, Hindi'], ['Product from Axom AI', 'Axomai Browser']],
    product_kicker='THE PRODUCT',
    product_h='What Axomai Browser is',
    product_p="A desktop browser for Windows 10 and 11. Web pages run on the Microsoft Edge WebView2 (Chromium) engine, so websites work the way they do in other modern browsers. The tabs, toolbar, settings and built-in pages are Axomai's own.",
    features=[['\U0001F916', 'Axom AI built in', 'Ask anything, or ask about the page you are reading. Answers stream in and chats are not saved.'],
              ['\U0001F5E3\uFE0F', 'Speaks your language', 'Menus and settings in Assamese, Hindi, Bengali and English, plus page translation and read-aloud.'],
              ['\U0001F33F', 'Made for Assam', 'Headlines from Assamese and North-East news sources, weather for 35 towns and five Assam-inspired themes.'],
              ['\U0001F6E1\uFE0F', 'Private by default', 'Ad and tracker blocking, HTTPS-only mode, per-site permissions and no account. The AI key stays on our server.'],
              ['\U0001F9F0', 'Everyday tools', 'Tab groups and search, split view, bookmarks, downloads, Chrome extensions (load unpacked) and encrypted folder sync.'],
              ['\U0001F513', 'Free and open', 'No price and no sign-in. Source code on GitHub under the MIT licence; updates are checked against a SHA-256 checksum.']],
    why_kicker='THE IDEA',
    why_h='Why Axom AI made a browser',
    why_p='Most people meet AI inside a browser tab. Putting Axom AI inside the browser means a student in Jorhat or a shopkeeper in Guwahati can ask a question in Assamese about the page in front of them, without copying text into another app.',
    why_quote='Axomai Browser is how Axom AI reaches everyday browsing: in your language, private by default, and free.',
    facts_kicker='AT A GLANCE',
    facts_h='Quick facts',
    facts=[['Product', 'Axomai Browser (also written Axom AI Browser)'], ['Parent company', 'Axom AI (aiaxom.co.in)'],
           ['Author', 'Samarjit Kashyap, lead AI researcher at Axom AI'], ['Platform', 'Windows 10 and 11 (64-bit)'], ['Version', '{version}'],
           ['Price', 'Free'], ['Licence', 'MIT, open source'], ['Interface languages', 'Assamese, Hindi, Bengali, English']],
    faq_kicker='FAQ',
    faq_h='Questions about us',
    faq=[
        ['Who makes Axomai Browser?',
         "Axomai Browser is made by Samarjit Kashyap, lead AI researcher at Axom AI, with Axom AI's founder Debajit Malakar. Axom AI (aiaxom.co.in) is an Assam-based artificial intelligence company."],
        ['Is Axomai Browser part of Axom AI?',
         'Yes. Axom AI (https://aiaxom.co.in/) is the parent company. Axomai Browser is one of its products, and it brings the Axom AI assistant into everyday web browsing.'],
        ['What is Axom AI?',
         "Axom AI is an Assam-based artificial intelligence platform in Guwahati built for Assamese speakers. Its website lists an Assamese chatbot, document and PDF tools, image generation, file utilities, web search, data analysis and translation, in Assamese, English and Hindi."],
        ['Is Axomai Browser free and open source?',
         'Yes. Axomai Browser is free to download and use, needs no account, and its source code is published on GitHub under the MIT licence.'],
        ['Where can I download Axomai Browser?',
         'From https://axomai-browser.aiaxom.co.in/. It runs on Windows 10 and Windows 11 (64-bit).'],
    ],
    band_h='Try Axomai Browser',
    band_p='Free for Windows 10 and 11. No account needed.',
    band_btn='Download for Windows',
)

# kept for the callers that import the old names
TITLE = ABOUT['title']
DESC = ABOUT['desc']
FAQ = ABOUT['faq']


def _e(s):
    return html.escape(s, quote=True)


def _rich(s, ctx):
    """Escapes text, then turns {parent} into a link to Axom AI and {version} into the version."""
    s = _e(s).replace('{version}', _e(ctx['version']))
    return s.replace('{parent}', '<a href="%s" rel="noopener">Axom AI</a>' % PARENT_URL)


def _linked(s, ctx):
    """Like _rich, and the text aiaxom.co.in becomes a link."""
    s = _rich(s, ctx)
    return s.replace('aiaxom.co.in', '<a href="%s" rel="noopener">aiaxom.co.in</a>' % PARENT_URL, 1) if '<a ' not in s else s


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


def body(ctx, dl, A):
    r = lambda k: _rich(A[k], ctx)
    faq = ''.join('<details%s><summary>%s</summary><p>%s</p></details>' % (' open' if i == 0 else '', _e(q), _e(a)) for i, (q, a) in enumerate(A['faq']))
    fc = ''.join('<div class="fcard"><span aria-hidden="true">%s</span><b>%s</b><p>%s</p></div>' % (_e(e), _e(t), _e(d)) for e, t, d in A['features'])
    chips = ''.join('<li>%s</li>' % _e(c) for c in A['chips'])
    people = ''.join('<div class="person"><i aria-hidden="true">%s</i><div><b>%s</b><span>%s</span></div></div>' % (_e(i), _e(n), _e(role)) for i, n, role in A['people'])
    glance = ''.join('<div><dt>%s</dt><dd>%s</dd></div>' % (_e(k), _linked(v, ctx)) for k, v in A['glance'])
    facts = ''.join('<tr><th scope="row">%s</th><td>%s</td></tr>' % (_e(k), _linked(v, ctx)) for k, v in A['facts'])
    return f'''<main id="main" class="about">
<div class="ahero"><div class="wrap"><div class="hgrid"><div>
<p class="crumbs"><a href="/">Axomai Browser</a> &nbsp;/&nbsp; {_e(A['crumb'])}</p>
<span class="eyebrow">{_e(A['eyebrow'])}</span>
<h1>{_e(A['h1_pre'])}<em>{_e(A['h1_em'])}</em></h1>
<p class="lead">{r('lead')}</p>
<ul class="chips">{chips}</ul>
<div class="acts"><a class="btn" href="{dl}" data-dl>{_e(A['btn_download'])}</a><a class="btn ghost" href="{PARENT_URL}" rel="noopener">{_e(A['btn_visit'])}</a></div>
</div>
<div class="family" aria-label="How Axom AI and Axomai Browser are related">
<div class="fam"><small>{_e(A['fam_parent_label'])}</small><b>Axom AI</b><p>{_e(A['fam_parent_p'])}</p></div>
<div class="arrow" aria-hidden="true">&rarr;</div>
<div class="fam main"><small>{_e(A['fam_product_label'])}</small><b>Axomai Browser</b><p>{_e(A['fam_product_p'])}</p></div>
</div></div></div></div>

<section id="parent" aria-labelledby="parent-h"><div class="wrap">
<div class="shead"><div class="kicker">{_e(A['company_kicker'])}</div><h2 id="parent-h">{_e(A['company_h'])}</h2></div>
<div class="parent">
<div>
<p>{r('company_p1')}</p>
<p>{r('company_p2')}</p>
<div class="people">{people}</div>
</div>
<aside class="pcard" aria-label="{_e(A['glance_h'])}"><h3>{_e(A['glance_h'])}</h3>
<dl>{glance}</dl></aside>
</div>
</div></section>

<section id="browser" class="alt" aria-labelledby="browser-h"><div class="wrap">
<div class="shead"><div class="kicker">{_e(A['product_kicker'])}</div><h2 id="browser-h">{_e(A['product_h'])}</h2>
<p>{r('product_p')}</p></div>
<div class="fgrid2">{fc}</div>
</div></section>

<section id="why" class="slim" aria-labelledby="why-h"><div class="wrap"><div class="why">
<div><div class="shead" style="margin:0"><div class="kicker">{_e(A['why_kicker'])}</div><h2 id="why-h">{_e(A['why_h'])}</h2>
<p>{r('why_p')}</p></div></div>
<div class="quote">{r('why_quote')}</div>
</div></div></section>

<section id="facts" class="alt" aria-labelledby="facts-h"><div class="wrap"><div class="duo">
<div><div class="shead"><div class="kicker">{_e(A['facts_kicker'])}</div><h2 id="facts-h">{_e(A['facts_h'])}</h2></div>
<table class="facts"><tbody>{facts}</tbody></table>
</div>

<div id="faq"><div class="shead"><div class="kicker">{_e(A['faq_kicker'])}</div><h2 id="faq-h">{_e(A['faq_h'])}</h2></div>
{faq}
</div>
</div></div></section>

<section class="slim"><div class="wrap"><div class="band"><h2>{_e(A['band_h'])}</h2><p>{r('band_p')}</p><a class="btn" href="{dl}" data-dl>{_e(A['band_btn'])}</a></div></div></section>
</main>'''


def graph(site, ctx, github, A):
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
        {'@type': 'AboutPage', '@id': about_url + '#webpage', 'url': about_url, 'name': A['title'], 'description': A['desc'], 'inLanguage': 'en-IN',
         'isPartOf': {'@id': site + '/#website'}, 'mainEntity': {'@id': PARENT_URL + '#org'}, 'dateModified': ctx['date'],
         'primaryImageOfPage': {'@type': 'ImageObject', 'url': site + '/assets/og-image.jpg'}},
        {'@type': 'WebSite', '@id': site + '/#website', 'url': site + '/', 'name': 'Axomai Browser', 'publisher': {'@id': site + '/#org'}},
        {'@type': 'SoftwareApplication', '@id': site + '/#app', 'name': 'Axomai Browser', 'applicationCategory': 'BrowserApplication', 'operatingSystem': 'Windows 10, Windows 11',
         'url': site + '/', 'author': {'@id': site + '/#org'}, 'publisher': {'@id': PARENT_URL + '#org'}, 'isAccessibleForFree': True,
         'offers': {'@type': 'Offer', 'price': '0', 'priceCurrency': 'INR'}},
        {'@type': 'FAQPage', '@id': about_url + '#faq', 'mainEntity': [{'@type': 'Question', 'name': q, 'acceptedAnswer': {'@type': 'Answer', 'text': a}} for q, a in A['faq']]},
        {'@type': 'BreadcrumbList', 'itemListElement': [{'@type': 'ListItem', 'position': 1, 'name': 'Axomai Browser', 'item': site + '/'},
                                                         {'@type': 'ListItem', 'position': 2, 'name': A['crumb'], 'item': about_url}]},
    ]


def build(home_html, site, ctx, github, dl, A=None):
    """Turns the rendered English landing page into the About page."""
    A = A or ABOUT
    about_url = site + '/about/'
    h = home_html
    e = html.escape

    def sub(pattern, repl, flags=0):
        nonlocal h
        new, n = re.subn(pattern, lambda m: repl, h, count=1, flags=flags)
        assert n == 1, pattern
        h = new

    sub(r'<title>.*?</title>', '<title>%s</title>' % e(A['title']))
    sub(r'<meta name="description" content="[^"]*">', '<meta name="description" content="%s">' % e(A['desc']))
    sub(r'<link rel="canonical" href="[^"]*">', '<link rel="canonical" href="%s">' % about_url)
    h = re.sub(r'<link rel="alternate" hreflang="[^"]*" href="[^"]*">', '', h)
    sub(r'<meta property="og:title" content="[^"]*">', '<meta property="og:title" content="%s">' % e(A['title']))
    sub(r'<meta property="og:description" content="[^"]*">', '<meta property="og:description" content="%s">' % e(A['desc']))
    sub(r'<meta property="og:url" content="[^"]*">', '<meta property="og:url" content="%s">' % about_url)
    sub(r'<meta name="twitter:title" content="[^"]*">', '<meta name="twitter:title" content="%s">' % e(A['title']))
    sub(r'<meta name="twitter:description" content="[^"]*">', '<meta name="twitter:description" content="%s">' % e(A['desc']))
    ld = json.dumps({'@context': 'https://schema.org', '@graph': graph(site, ctx, github, A)}, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
    sub(r'<script type="application/ld\+json">.*?</script>', '<script type="application/ld+json">%s</script>' % ld, re.S)
    sub(r'</style>', CSS + '</style>')
    sub(r'<main id="main">.*?</main>', body(ctx, dl, A), re.S)
    # the header, menu and footer links point at sections of the home page
    h = h.replace('<a href="/about/">', '<a href="/about/" aria-current="page">')
    h = h.replace('href="#', 'href="/#')
    h = h.replace('href="/#main"', 'href="#main"')
    return h
