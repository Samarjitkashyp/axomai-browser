<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:sm="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
<xsl:output method="html" encoding="UTF-8" indent="yes" doctype-system="about:legacy-compat"/>
<xsl:template match="/">
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><meta name="robots" content="noindex"/>
<title>Sitemap - Axomai Browser</title>
<style>
:root{--acc:#10b981;--bg:#f0fdf4;--card:#fff;--head:#022c22;--text:#064e3b;--muted:#5b6b66;--line:#cfe9dc}
@media (prefers-color-scheme:dark){:root{--bg:#0b1220;--card:#111c2e;--head:#fff;--text:#e2e8f0;--muted:#94a3b8;--line:#1f2f46}}
*{box-sizing:border-box}body{margin:0;font-family:'Plus Jakarta Sans',-apple-system,'Segoe UI',Roboto,sans-serif;background:var(--bg);color:var(--text);line-height:1.6}
.wrap{max-width:980px;margin:0 auto;padding:32px 16px 48px}
h1{margin:0 0 6px;font-size:1.7rem;color:var(--head)}p{color:var(--muted);margin:0 0 22px}
.brand{display:inline-block;font-weight:800;color:var(--acc);margin-bottom:10px;text-decoration:none}
.card{background:var(--card);border:1px solid var(--line);border-radius:16px;overflow:hidden}
table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:12px 14px;border-bottom:1px solid var(--line);font-size:.92rem;vertical-align:top}
th{font-size:.75rem;letter-spacing:.06em;text-transform:uppercase;color:var(--muted);background:rgba(16,185,129,.08)}
tr:last-child td{border-bottom:0}a{color:var(--acc);font-weight:600;word-break:break-all}
.pill{display:inline-block;background:rgba(16,185,129,.14);border-radius:999px;padding:1px 9px;font-size:.78rem;margin:0 4px 4px 0}
@media (max-width:640px){th:nth-child(n+3),td:nth-child(n+3){display:none}}
</style></head>
<body><div class="wrap">
<a class="brand" href="/">Axomai Browser</a>
<h1>Sitemap</h1>
<p>This is the sitemap search engines read. It lists <b><xsl:value-of select="count(sm:urlset/sm:url)"/></b> page<xsl:if test="count(sm:urlset/sm:url)!=1">s</xsl:if>.</p>
<div class="card"><table>
<tr><th>Page</th><th>Languages</th><th>Last changed</th><th>Updates</th><th>Priority</th></tr>
<xsl:for-each select="sm:urlset/sm:url">
<tr>
<td><a href="{sm:loc}"><xsl:value-of select="sm:loc"/></a></td>
<td><xsl:for-each select="xhtml:link"><span class="pill"><xsl:value-of select="@hreflang"/></span></xsl:for-each></td>
<td><xsl:value-of select="sm:lastmod"/></td>
<td><xsl:value-of select="sm:changefreq"/></td>
<td><xsl:value-of select="sm:priority"/></td>
</tr></xsl:for-each>
</table></div>
</div></body></html>
</xsl:template></xsl:stylesheet>
