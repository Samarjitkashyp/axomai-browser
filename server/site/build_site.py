# -*- coding: utf-8 -*-
"""Builds the Axomai Browser landing page (English and Assamese), robots.txt, sitemap.xml, llms.txt and the web manifest.
   python build_site.py [--version 4.1.0] [--size 28] [--sha <sha256>] [--out <folder>]
The output folder is what gets uploaded to /var/www/axomai-browser/site/ on the server."""
import argparse, html, json, os, re, shutil
import content as C

HERE = os.path.dirname(os.path.abspath(__file__))

def esc(s):
    return html.escape(s, quote=True)

def ra(s):
    """Assamese writes ra as U+09F0, not the Bengali U+09B0."""
    return s.replace('র', 'ৰ')

def localised(lang):
    d = dict(C.EN if lang == 'en' else C.AS)
    if lang == 'as':
        def walk(v):
            if isinstance(v, str):
                return ra(v)
            if isinstance(v, (list, tuple)):
                return type(v)(walk(x) for x in v)
            if isinstance(v, dict):
                return {k: walk(x) for k, x in v.items()}
            return v
        d = {k: walk(v) for k, v in d.items()}
    return d

def fmt(s, ctx):
    return s.replace('{version}', ctx['version']).replace('{size}', ctx['size']).replace('{sha}', ctx['sha'])

def strip_tags(s):
    return re.sub(r'<[^>]+>', '', html.unescape(s))

def page(lang, ctx):
    t = localised(lang)
    css = open(os.path.join(HERE, 'site.css'), encoding='utf-8').read()
    url = C.SITE + ('/' if lang == 'en' else '/as/')
    dl = '/downloads/Axomai-Setup-%s.exe' % ctx['version']
    title, desc = fmt(t['title'], ctx), fmt(t['desc'], ctx)
    reviewed = C.REVIEWED[lang]
    robots = 'index,follow,max-snippet:-1,max-image-preview:large,max-video-preview:-1' if reviewed else 'noindex,follow'
    alts = ''
    if reviewed:
        alts = ''.join('<link rel="alternate" hreflang="%s" href="%s%s">' % (l, C.SITE, p) for l, _, p in C.LANGS if C.REVIEWED[l])
        alts += '<link rel="alternate" hreflang="x-default" href="%s/">' % C.SITE

    # ---- structured data (one graph)
    faq = [(q, strip_tags(a)) for q, a in t['faq']]
    steps = [(n, strip_tags(fmt(d, ctx))) for n, d in t['steps']]
    features_flat = [b for f in t['features'] for b in f['bullets']]
    graph = [
        {'@type': 'Organization', '@id': C.SITE + '/#org', 'name': 'Axomai', 'url': C.SITE + '/', 'logo': {'@type': 'ImageObject', 'url': C.SITE + '/assets/logo-512.png', 'width': 512, 'height': 512},
         'founder': {'@type': 'Person', 'name': 'Samarjit Kashyap'}, 'sameAs': [C.GITHUB], 'areaServed': {'@type': 'AdministrativeArea', 'name': 'Assam, India'}},
        {'@type': 'WebSite', '@id': C.SITE + '/#website', 'url': C.SITE + '/', 'name': 'Axomai Browser', 'publisher': {'@id': C.SITE + '/#org'}, 'inLanguage': [l for l, _, _ in C.LANGS if C.REVIEWED[l]]},
        {'@type': 'WebPage', '@id': url + '#webpage', 'url': url, 'name': title, 'description': desc, 'isPartOf': {'@id': C.SITE + '/#website'}, 'about': {'@id': C.SITE + '/#app'},
         'primaryImageOfPage': {'@type': 'ImageObject', 'url': C.SITE + '/assets/og-image.jpg'}, 'inLanguage': C.HTML_LANG[lang], 'dateModified': ctx['date']},
        {'@type': 'SoftwareApplication', '@id': C.SITE + '/#app', 'name': 'Axomai Browser', 'alternateName': ['Axom AI Browser', 'Axomai', 'Axom AI', 'আক্সমাই ব্ৰাউজাৰ'],
         'description': strip_tags(C.EN['def_p']), 'applicationCategory': 'BrowserApplication', 'operatingSystem': 'Windows 10, Windows 11', 'softwareVersion': ctx['version'],
         'fileSize': ctx['size'] + ' MB', 'downloadUrl': C.SITE + dl, 'installUrl': C.SITE + dl, 'url': C.SITE + '/', 'image': C.SITE + '/assets/og-image.jpg', 'screenshot': C.SITE + '/assets/og-image.jpg',
         'inLanguage': ['as', 'hi', 'bn', 'en'], 'datePublished': '2026-10-05', 'dateModified': ctx['date'], 'isAccessibleForFree': True,
         'offers': {'@type': 'Offer', 'price': '0', 'priceCurrency': 'INR', 'availability': 'https://schema.org/InStock', 'url': C.SITE + '/'},
         'author': {'@id': C.SITE + '/#org'}, 'publisher': {'@id': C.SITE + '/#org'}, 'featureList': features_flat, 'codeRepository': C.GITHUB},
        {'@type': 'FAQPage', '@id': url + '#faq', 'mainEntity': [{'@type': 'Question', 'name': q, 'acceptedAnswer': {'@type': 'Answer', 'text': a}} for q, a in faq]},
        {'@type': 'HowTo', '@id': url + '#howto', 'name': t['howto_name'], 'totalTime': 'PT2M', 'tool': [{'@type': 'HowToTool', 'name': 'Windows 10 or 11 PC'}],
         'step': [{'@type': 'HowToStep', 'position': i + 1, 'name': n, 'text': d} for i, (n, d) in enumerate(steps)]},
        {'@type': 'BreadcrumbList', 'itemListElement': [{'@type': 'ListItem', 'position': 1, 'name': 'Axomai Browser', 'item': C.SITE + '/'}] + ([] if lang == 'en' else [{'@type': 'ListItem', 'position': 2, 'name': C.LANGS[1][1], 'item': url}])},
    ]
    ld = json.dumps({'@context': 'https://schema.org', '@graph': graph}, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')

    # ---- body pieces
    nav = ''.join('<a href="%s">%s</a>' % (h, esc(n)) for h, n in t['nav'])
    langs = ''.join('<a href="%s" hreflang="%s" lang="%s"%s>%s</a>' % (p, l, l, ' aria-current="true"' if l == lang else '', esc(n)) for l, n, p in C.LANGS)
    chat = ''.join('<div class="msg %s">%s</div>' % (r, esc(m)) for r, m in t['chat'])
    strip = ''.join('<div>%s<small>%s</small></div>' % (esc(a), esc(b)) for a, b in t['strip'])
    facts = ''.join('<tr><th scope="row">%s</th><td>%s</td></tr>' % (esc(k), esc(fmt(v, ctx))) for k, v in t['facts'])
    feats = ''
    for i, f in enumerate(t['features']):
        bullets = ''.join('<li>%s</li>' % esc(b) for b in f['bullets'])
        vis = ''.join('<div class="row"><span aria-hidden="true">%s</span>%s</div>' % (e, esc(x)) for e, x in f['vis'])
        feats += ('<div class="feat%s" id="%s"><div class="copy"><div class="kicker">%s</div><h3>%s</h3><p>%s</p><ul>%s</ul></div><div class="vis" role="img" aria-label="%s">%s</div></div>'
                  % (' rev' if i % 2 else '', f['id'], esc(f['kicker']), esc(f['h']), esc(f['p']), bullets, esc(f['h']), vis))
    themes = ''.join('<div><i style="background:linear-gradient(135deg,%s)"></i><span>%s</span></div>' % (c, esc(n)) for n, c in t['themes'])
    gallery = ''.join('<figure><img src="/assets/%s" alt="%s" width="1000" height="558" loading="lazy" decoding="async"><figcaption>%s</figcaption></figure>' % (img, esc(alt), esc(cap)) for img, alt, cap in t['gallery'][1:])
    steps_html = ''.join('<li><h3>%s</h3><p>%s</p></li>' % (esc(n), fmt(d, ctx)) for n, d in t['steps'])
    req = ''.join('<li>%s</li>' % esc(r) for r in t['req'])
    faq_html = ''.join('<details%s><summary>%s</summary><p>%s</p></details>' % (' open' if i == 0 else '', esc(q), esc(fmt(a, ctx))) for i, (q, a) in enumerate(t['faq']))
    flinks = ''.join('<a href="%s">%s</a>' % (esc(fmt(h, ctx)), esc(n)) for n, h in t['footer_links'])
    cta_meta = fmt(t['cta_meta'], ctx)

    return f'''<!doctype html>
<html lang="{C.HTML_LANG[lang]}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{esc(title)}</title>
<meta name="description" content="{esc(desc)}">
<meta name="robots" content="{robots}">
<meta name="keywords" content="Axomai Browser, Axom AI, AI browser, artificial intelligence browser, Assam browser, Assamese browser, Assamese AI, free browser for Windows, AI chat browser">
<meta name="author" content="Samarjit Kashyap">
<link rel="canonical" href="{url}">
{alts}
<meta name="theme-color" content="#059669">
<link rel="icon" href="/favicon.ico" sizes="any"><link rel="icon" type="image/png" sizes="32x32" href="/assets/favicon-32.png"><link rel="apple-touch-icon" href="/assets/apple-touch-icon.png"><link rel="manifest" href="/site.webmanifest">
<meta property="og:type" content="website"><meta property="og:site_name" content="Axomai Browser"><meta property="og:title" content="{esc(title)}"><meta property="og:description" content="{esc(desc)}">
<meta property="og:url" content="{url}"><meta property="og:image" content="{C.SITE}/assets/og-image.jpg"><meta property="og:image:width" content="1200"><meta property="og:image:height" content="630">
<meta property="og:image:alt" content="Axomai Browser, the free AI browser made in Assam"><meta property="og:locale" content="{C.OG_LOCALE[lang]}">
<meta name="twitter:card" content="summary_large_image"><meta name="twitter:title" content="{esc(title)}"><meta name="twitter:description" content="{esc(desc)}"><meta name="twitter:image" content="{C.SITE}/assets/og-image.jpg">
<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=Noto+Sans+Bengali:wght@400;600;700&display=swap" rel="stylesheet">
<style>{css}</style>
<script type="application/ld+json">{ld}</script>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>
<header class="top"><div class="wrap nav">
<a class="brand" href="{'/' if lang == 'en' else '/as/'}"><img src="/assets/logo-192.png" alt="Axomai Browser logo" width="34" height="34"><span class="bt">Axomai Browser</span></a>
<nav aria-label="Main">{nav}</nav>
<div class="langs" aria-label="Language">{langs}</div>
<a class="btn sm" href="{dl}" data-dl><span class="full">{esc(t['cta'])}</span><span class="short">{esc(t['cta_short'])}</span></a>
</div></header>
<main id="main">
<div class="hero"><div class="wrap">
<span class="eyebrow">{t['eyebrow']}</span>
<h1>{t['h1']}</h1>
<p class="lead">{esc(t['lead'])}</p>
<div class="ctas"><a class="btn" href="{dl}" data-dl><span aria-hidden="true">⬇</span>{esc(t['cta'])} <small style="opacity:.85;font-weight:600">{esc(cta_meta)}</small></a><a class="btn ghost" href="#ai">{esc(t['secondary'])}</a></div>
<p class="fine">{fmt(t['fine'], ctx)}</p>
<div class="nowin-note" role="note"><b>{esc(t['nowin_h'])}</b><br>{esc(t['nowin_p'])}<br><button class="btn sm" type="button" id="share">{esc(t['nowin_btn'])}</button></div>
<div class="shot" role="img" aria-label="Axomai Browser window with the Axom AI chat open beside a web page">
<div class="tabs"><div class="tab"><span class="dot"></span>{esc(t['shot_title'])}</div><div class="tab off">New Tab</div></div>
<div class="addr"><span aria-hidden="true">← → ↻</span><div class="pill">{esc(t['shot_url'])}</div><span class="ai-chip">AI</span></div>
<div class="stage"><div class="page"><b>{esc(t['shot_title'])}</b><span>{esc(t['shot_sub'])}</span></div><div class="panel">{chat}</div></div>
</div>
<div class="strip">{strip}</div>
</div></div>
<section class="alt"><div class="wrap"><div class="kicker">{esc(t['def_kicker'])}</div><h2>{esc(t['def_h'])}</h2><p class="answer">{t['def_p']}</p>
<h3 style="margin:26px 0 0;color:var(--head)">{esc(t['facts_h'])}</h3><table class="facts"><tbody>{facts}</tbody></table></div></section>
<section><div class="wrap">{feats}</div></section>
<section class="alt"><div class="wrap"><h2>{esc(t['themes_h'])}</h2><p>{esc(t['themes_p'])}</p><div class="sw">{themes}</div>
<h2 style="margin-top:56px">{esc(t['gallery_h'])}</h2><p><small>{esc(t['gallery_note'])}</small></p><div class="gal">{gallery}</div></div></section>
<section id="install"><div class="wrap"><h2>{esc(t['install_h'])}</h2><ol class="steps">{steps_html}</ol>
<h3 style="margin-top:34px;color:var(--head)">{esc(t['req_h'])}</h3><ul>{req}</ul></div></section>
<section class="alt" id="faq"><div class="wrap"><h2>{esc(t['faq_h'])}</h2>{faq_html}</div></section>
<section><div class="wrap"><div class="band"><h2>{esc(t['final_h'])}</h2><p>{esc(t['final_p'])}</p><a class="btn" href="{dl}" data-dl>{esc(t['cta'])}</a></div></div></section>
</main>
<footer><div class="wrap cols"><span>{esc(t['footer_note'])}</span><span>{flinks}</span></div></footer>
<script>
// On a phone or a Mac the Windows installer is of no use: offer to send the link to a Windows PC instead.
(function(){{
  if(/Windows NT/.test(navigator.userAgent))return;
  document.documentElement.classList.add('nowin');
  var b=document.getElementById('share');if(!b)return;
  b.addEventListener('click',function(){{
    var data={{title:document.title,text:'Axomai Browser for Windows',url:location.origin+location.pathname}};
    if(navigator.share){{navigator.share(data).catch(function(){{}})}}
    else if(navigator.clipboard){{navigator.clipboard.writeText(data.url).then(function(){{b.textContent='\u2713'}})}}
  }});
}})();
// Keeps the download button and checksum on the newest release without rebuilding this page.
fetch('/downloads/release.json',{{cache:'no-store'}}).then(function(r){{if(!r.ok)throw 0;return r.json()}}).then(function(j){{
  var exe=(j.assets||[]).filter(function(a){{return /^Axomai-Setup-.*\\.exe$/.test(a.name)}})[0];if(!exe)return;
  document.querySelectorAll('[data-dl]').forEach(function(a){{a.href=exe.browser_download_url}});
  var sha=(j.assets||[]).filter(function(a){{return /\\.sha256$/.test(a.name)}})[0];
  if(sha)fetch(sha.browser_download_url,{{cache:'no-store'}}).then(function(r){{return r.text()}}).then(function(t){{var h=(t.trim().split(/\\s+/)[0]||'').toLowerCase();if(/^[0-9a-f]{{64}}$/.test(h)){{var el=document.getElementById('sha');if(el)el.textContent=h}}}});
}}).catch(function(){{}});
</script>
</body>
</html>
'''

def robots():
    bots = ['GPTBot', 'ChatGPT-User', 'OAI-SearchBot', 'ClaudeBot', 'Claude-User', 'Claude-SearchBot', 'PerplexityBot', 'Perplexity-User', 'Google-Extended', 'Applebot-Extended', 'CCBot', 'Bytespider', 'Amazonbot']
    out = 'User-agent: *\nAllow: /\nDisallow: /v1/\n\n'
    out += ''.join('User-agent: %s\nAllow: /\n\n' % b for b in bots)
    return out + 'Sitemap: %s/sitemap.xml\n' % C.SITE

def sitemap(date):
    live = [(l, p) for l, _, p in C.LANGS if C.REVIEWED[l]]
    urls = ''
    for l, p in live:
        alts = ''.join('<xhtml:link rel="alternate" hreflang="%s" href="%s%s"/>' % (l2, C.SITE, p2) for l2, p2 in live) + '<xhtml:link rel="alternate" hreflang="x-default" href="%s/"/>' % C.SITE
        urls += '<url><loc>%s%s</loc><lastmod>%s</lastmod><changefreq>weekly</changefreq><priority>%s</priority>%s</url>' % (C.SITE, p, date, '1.0' if p == '/' else '0.8', alts)
    return '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">%s</urlset>\n' % urls

def llms(ctx, full):
    t = C.EN
    lines = ['# Axomai Browser', '', '> ' + strip_tags(t['def_p']), '',
             'Axomai Browser (also written Axom AI Browser) is a free Windows web browser with a built-in AI assistant, made in Assam, India by Samarjit Kashyap.', '', '## Facts']
    lines += ['- %s: %s' % (k, fmt(v, ctx)) for k, v in t['facts']]
    lines += ['', '## Links', '- Download page: %s/' % C.SITE, '- Installer (Windows, %s MB): %s/downloads/Axomai-Setup-%s.exe' % (ctx['size'], C.SITE, ctx['version']), '- SHA-256 checksum: %s/downloads/Axomai-Setup-%s.exe.sha256' % (C.SITE, ctx['version']), '- Source code: ' + C.GITHUB]
    if full:
        lines += ['', '## Features']
        for f in t['features']:
            lines += ['', '### ' + f['h'], f['p']] + ['- ' + b for b in f['bullets']]
        lines += ['', '## Frequently asked questions']
        for q, a in t['faq']:
            lines += ['', '### ' + q, strip_tags(a)]
        lines += ['', '## Install', ''] + ['%d. %s: %s' % (i + 1, n, strip_tags(fmt(d, ctx))) for i, (n, d) in enumerate(t['steps'])]
    return '\n'.join(lines) + '\n'

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--version', default='4.1.0'); ap.add_argument('--size', default='28'); ap.add_argument('--date', default='2026-10-05')
    ap.add_argument('--sha', default='see the .sha256 file next to the installer'); ap.add_argument('--out', default=os.path.join(HERE, 'dist'))
    a = ap.parse_args()
    ctx = dict(version=a.version, size=a.size, sha=a.sha, date=a.date)
    out = a.out
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(os.path.join(out, 'as'))
    shutil.copytree(os.path.join(HERE, 'assets'), os.path.join(out, 'assets'))
    shutil.copy(os.path.join(HERE, 'favicon.ico'), out)
    w = lambda p, s: open(os.path.join(out, p), 'w', encoding='utf-8', newline='\n').write(s)
    w('index.html', page('en', ctx)); w('as/index.html', page('as', ctx))
    w('robots.txt', robots()); w('sitemap.xml', sitemap(a.date)); w('llms.txt', llms(ctx, False)); w('llms-full.txt', llms(ctx, True))
    w('site.webmanifest', json.dumps({'name': 'Axomai Browser', 'short_name': 'Axomai', 'description': 'The free AI browser made in Assam', 'start_url': '/', 'display': 'browser', 'background_color': '#f0fdf4', 'theme_color': '#059669',
                                      'icons': [{'src': '/assets/logo-192.png', 'sizes': '192x192', 'type': 'image/png'}, {'src': '/assets/logo-512.png', 'sizes': '512x512', 'type': 'image/png'}]}, indent=2))
    print('built', out, sorted(os.listdir(out)))

if __name__ == '__main__':
    main()
