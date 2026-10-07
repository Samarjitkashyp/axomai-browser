# -*- coding: utf-8 -*-
"""Applies the crawl-option fixes to the bot's code (run once, from the bot folder: python3 patch_bot.py).
  1. crawler/frontier.py : "Include Subdomains" no longer treats every .gov.in / .co.in / .co.uk site as the same site
  2. crawler/engine.py   : a site profile with allow_subdomains=false switches Include Subdomains off for that site
  3. app/main.py         : the import into Axom AI goes in batches of 200 pages, with retries (app/chat_import.py)
"""
import re
import sys


def patch(path, edits, marker):
    s = open(path, encoding='utf-8').read()
    assert marker not in s, path + ' already patched'
    for old, new in edits:
        assert old in s, (path, old[:70])
        s = s.replace(old, new, 1)
    open(path, 'w', encoding='utf-8').write(s)
    print('patched', path)


patch('crawler/frontier.py', [(
    '''        parts = netloc.replace("www.", "").split(".")
        if len(parts) >= 2:
            return ".".join(parts[-2:])
        return netloc''',
    '''        parts = netloc.replace("www.", "").split(".")
        # AXOMAI-FIX: under co.in, gov.in, org.in, co.uk ... the registrable domain has THREE labels (assam.gov.in), otherwise
        # every .gov.in or .co.in site would count as the same site when "Include Subdomains" is on
        if len(parts) >= 3 and len(parts[-1]) == 2 and parts[-2] in ("co", "com", "org", "net", "gov", "edu", "ac", "res", "mil", "ind", "gen", "firm", "or", "ne", "go"):
            return ".".join(parts[-3:])
        if len(parts) >= 2:
            return ".".join(parts[-2:])
        return netloc''')], 'AXOMAI-FIX')

patch('crawler/engine.py', [(
    '''        self.job = job
        self.storage = storage_manager or StorageManager()''',
    '''        self.job = job
        # AXOMAI-LOCK: a site whose profile (crawler/site_profiles.json) says allow_subdomains=false is never crawled across subdomains
        try:
            from crawler.cleaner import load_profiles
            from urllib.parse import urlparse as _urlparse
            _host = _urlparse(job.seed_url).netloc.lower().replace("www.", "")
            _prof = load_profiles().get(_host)
            if job.include_subdomains and isinstance(_prof, dict) and _prof.get("allow_subdomains") is False:
                job.include_subdomains = False
                job.log("Include Subdomains was switched off: the profile of %s does not allow it" % _host)
        except Exception:
            pass
        self.storage = storage_manager or StorageManager()''')], 'AXOMAI-LOCK')

s = open('app/main.py', encoding='utf-8').read()
assert 'chat_import' not in s, 'app/main.py already patched'
a = s.index('async def _push_to_chat(job_id: str, seed_url: str):')
b = s.index('# --- Crawler Endpoints ---')
new_fn = '''async def _push_to_chat(job_id: str, seed_url: str):
    if not BOT_IMPORT_TOKEN:
        _import_logger.warning("BOT_IMPORT_TOKEN not set — skipping chat import")
        return
    data = storage_manager.get_crawl_job(job_id)
    if not data:
        return
    pages = data.get("pages", [])
    if not pages:
        return
    # AXOMAI-BATCH: batches of 200 pages with retries (a single huge request could time out and lose the whole crawl)
    from app.chat_import import push_pages
    await push_pages(CHAT_IMPORT_URL, BOT_IMPORT_TOKEN, seed_url, job_id, pages)


'''
s = s[:a] + new_fn + s[b:]
open('app/main.py', 'w', encoding='utf-8').write(s)
print('patched app/main.py')
