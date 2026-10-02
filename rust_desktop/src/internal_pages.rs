use crate::{Extension, SearchEngine};

pub fn home_page_html_with_engine(search_url_base: &str) -> String {
    home_page_html_inner(search_url_base)
}

#[allow(dead_code)]
pub fn home_page_html() -> String {
    home_page_html_inner("https://www.google.com/search?q=")
}

fn home_page_html_inner(search_url_base: &str) -> String {
    let template = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#f8f9fc;--fg:#1a1d2e;--muted:#6b7085;
  --surface:#ffffff;--border:#e4e7f0;
  --accent:#4f46e5;--accent-light:#eef2ff;
  --radius:14px;--shadow:0 2px 12px rgba(0,0,0,.06);
}
@media(prefers-color-scheme:dark){
  :root{
    --bg:#0c0d14;--fg:#e4e6f0;--muted:#7a7f96;
    --surface:#16171f;--border:#252736;
    --accent:#818cf8;--accent-light:rgba(129,140,248,.12);
    --shadow:0 2px 12px rgba(0,0,0,.3);
    color-scheme:dark;
  }
}
body{
  background:var(--bg);color:var(--fg);
  font-family:'Segoe UI',system-ui,-apple-system,sans-serif;
  min-height:100vh;display:flex;flex-direction:column;
  align-items:center;justify-content:center;
  padding:40px 24px;
  -webkit-user-select:none;user-select:none;
}
.logo{
  width:72px;height:72px;
  background:linear-gradient(135deg,#4f46e5,#7c3aed);
  border-radius:18px;display:flex;align-items:center;justify-content:center;
  color:#fff;font-size:36px;font-weight:800;
  margin-bottom:24px;
  box-shadow:0 8px 32px rgba(79,70,229,.25);
}
h1{
  font-size:22px;font-weight:700;margin-bottom:6px;
  letter-spacing:-.02em;
}
.tagline{color:var(--muted);font-size:14px;margin-bottom:32px}
.search-box{
  width:min(100%,560px);position:relative;margin-bottom:40px;
}
.search-box input{
  width:100%;padding:16px 20px 16px 48px;
  background:var(--surface);color:var(--fg);
  border:1px solid var(--border);border-radius:28px;
  font-size:15px;outline:none;
  box-shadow:var(--shadow);
  transition:border-color .2s,box-shadow .2s;
}
.search-box input:focus{
  border-color:var(--accent);
  box-shadow:0 0 0 3px var(--accent-light),var(--shadow);
}
.search-box input::placeholder{color:var(--muted)}
.search-icon{
  position:absolute;left:18px;top:50%;transform:translateY(-50%);
  color:var(--muted);pointer-events:none;
}
.shortcuts{
  display:grid;
  grid-template-columns:repeat(auto-fit,minmax(88px,1fr));
  gap:16px;width:min(100%,560px);margin-bottom:40px;
}
.shortcut{
  display:flex;flex-direction:column;align-items:center;gap:10px;
  padding:16px 8px;border-radius:var(--radius);
  background:var(--surface);border:1px solid var(--border);
  cursor:pointer;transition:transform .15s,border-color .2s,box-shadow .2s;
  text-decoration:none;color:var(--fg);
}
.shortcut:hover{
  transform:translateY(-2px);border-color:var(--accent);
  box-shadow:var(--shadow);
}
.shortcut-icon{
  width:44px;height:44px;border-radius:12px;
  display:flex;align-items:center;justify-content:center;
  font-size:20px;font-weight:700;color:#fff;
}
.shortcut-label{font-size:12px;font-weight:500;color:var(--muted)}
.footer-info{
  color:var(--muted);font-size:12px;
  display:flex;align-items:center;gap:6px;
}
.footer-info .dot{width:4px;height:4px;border-radius:50%;background:var(--muted)}
</style>
</head>
<body>
<div class="logo">A</div>
<h1>Axomai Browser</h1>
<p class="tagline">GPU-accelerated browsing, built with Rust</p>
<div class="search-box">
  <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
  <input type="text" id="searchInput" placeholder="Search the web or enter a URL..." autofocus>
</div>
<div class="shortcuts">
  <a class="shortcut" data-url="https://www.google.com">
    <div class="shortcut-icon" style="background:#4285f4">G</div>
    <span class="shortcut-label">Google</span>
  </a>
  <a class="shortcut" data-url="https://www.youtube.com">
    <div class="shortcut-icon" style="background:#ff0000">Y</div>
    <span class="shortcut-label">YouTube</span>
  </a>
  <a class="shortcut" data-url="https://github.com">
    <div class="shortcut-icon" style="background:#24292f">G</div>
    <span class="shortcut-label">GitHub</span>
  </a>
  <a class="shortcut" data-url="https://twitter.com">
    <div class="shortcut-icon" style="background:#1da1f2">X</div>
    <span class="shortcut-label">Twitter</span>
  </a>
  <a class="shortcut" data-url="https://www.reddit.com">
    <div class="shortcut-icon" style="background:#ff4500">R</div>
    <span class="shortcut-label">Reddit</span>
  </a>
</div>
<div class="footer-info">
  <span>Axomai v1.0</span>
  <span class="dot"></span>
  <span>GPU Rendered</span>
  <span class="dot"></span>
  <span>Rust Powered</span>
</div>
<script>
document.getElementById('searchInput').addEventListener('keydown',function(e){
  if(e.key==='Enter'&&this.value.trim()){
    var q=this.value.trim();
    var url;
    if(q.includes('://')||(/^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}/.test(q)&&!q.includes(' '))){
      url=q.includes('://')?q:'https://'+q;
    }else{
      url='SEARCH_ENGINE_URL_PLACEHOLDER'+encodeURIComponent(q);
    }
    window.location.href=url;
  }
});
document.querySelectorAll('.shortcut').forEach(function(el){
  el.addEventListener('click',function(){
    window.location.href=this.dataset.url;
  });
});
</script>
</body>
</html>"##;
    template.replace("SEARCH_ENGINE_URL_PLACEHOLDER", search_url_base)
}

pub fn extensions_page_html(extensions: &[Extension]) -> String {
    let mut ext_cards = String::new();
    for (i, ext) in extensions.iter().enumerate() {
        let status_class = if ext.enabled { "active" } else { "" };
        let status_text = if ext.enabled { "Active" } else { "Inactive" };
        let badge = if ext.auto_inject { r#"<span class="badge">Auto</span>"# } else { "" };
        let toggle_checked = if ext.enabled { "checked" } else { "" };
        let r = ext.icon_color[0];
        let g = ext.icon_color[1];
        let b = ext.icon_color[2];
        ext_cards.push_str(&format!(
            r##"<div class="ext-card">
  <div class="ext-header">
    <div class="ext-icon" style="background:rgb({r},{g},{b})">{letter}</div>
    <div class="ext-info">
      <h3>{name}{badge}</h3>
      <p class="ext-version">v{version}</p>
    </div>
    <label class="toggle">
      <input type="checkbox" data-idx="{idx}" {checked}>
      <span class="slider"></span>
    </label>
  </div>
  <p class="ext-desc">{desc}</p>
  <div class="ext-footer">
    <span class="status {status_class}">{status}</span>
  </div>
</div>"##,
            r=r, g=g, b=b,
            letter=ext.icon_letter,
            name=ext.name,
            badge=badge,
            version=ext.version,
            idx=i,
            checked=toggle_checked,
            desc=ext.description,
            status_class=status_class,
            status=status_text,
        ));
    }

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg:#f8f9fc;--fg:#1a1d2e;--muted:#6b7085;
  --surface:#ffffff;--border:#e4e7f0;
  --accent:#4f46e5;--accent-light:#eef2ff;
  --green:#22c55e;--green-light:#dcfce7;
  --radius:14px;--shadow:0 2px 12px rgba(0,0,0,.06);
}}
@media(prefers-color-scheme:dark){{
  :root{{
    --bg:#0c0d14;--fg:#e4e6f0;--muted:#7a7f96;
    --surface:#16171f;--border:#252736;
    --accent:#818cf8;--accent-light:rgba(129,140,248,.12);
    --green:#4ade80;--green-light:rgba(74,222,128,.12);
    --shadow:0 2px 12px rgba(0,0,0,.3);
    color-scheme:dark;
  }}
}}
body{{
  background:var(--bg);color:var(--fg);
  font-family:'Segoe UI',system-ui,-apple-system,sans-serif;
  padding:32px 28px;
  -webkit-user-select:none;user-select:none;
}}
.page-header{{margin-bottom:28px}}
.page-header h1{{font-size:24px;font-weight:700;letter-spacing:-.02em;margin-bottom:4px}}
.page-header p{{color:var(--muted);font-size:14px}}
.ext-grid{{
  display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));
  gap:16px;
}}
.ext-card{{
  background:var(--surface);border:1px solid var(--border);
  border-radius:var(--radius);padding:20px;
  transition:border-color .2s,transform .15s;
}}
.ext-card:hover{{border-color:var(--accent);transform:translateY(-1px)}}
.ext-header{{display:flex;align-items:center;gap:14px;margin-bottom:12px}}
.ext-icon{{
  width:42px;height:42px;border-radius:11px;flex-shrink:0;
  display:flex;align-items:center;justify-content:center;
  color:#fff;font-size:18px;font-weight:700;
}}
.ext-info{{flex:1;min-width:0}}
.ext-info h3{{font-size:15px;font-weight:600;display:flex;align-items:center;gap:8px}}
.ext-version{{color:var(--muted);font-size:12px}}
.badge{{
  background:var(--accent-light);color:var(--accent);
  padding:2px 8px;border-radius:100px;font-size:11px;font-weight:600;
}}
.ext-desc{{color:var(--muted);font-size:13px;line-height:1.5;margin-bottom:14px}}
.ext-footer{{display:flex;align-items:center;justify-content:space-between}}
.status{{
  font-size:12px;font-weight:600;
  padding:4px 12px;border-radius:100px;
  color:var(--muted);background:var(--bg);
}}
.status.active{{color:var(--green);background:var(--green-light)}}
.toggle{{position:relative;width:44px;height:24px;flex-shrink:0}}
.toggle input{{opacity:0;width:0;height:0}}
.slider{{
  position:absolute;cursor:pointer;
  top:0;left:0;right:0;bottom:0;
  background:var(--border);border-radius:24px;
  transition:.25s;
}}
.slider::before{{
  content:'';position:absolute;
  height:18px;width:18px;left:3px;bottom:3px;
  background:#fff;border-radius:50%;
  transition:.25s;
}}
input:checked+.slider{{background:var(--accent)}}
input:checked+.slider::before{{transform:translateX(20px)}}
</style>
</head>
<body>
<div class="page-header">
  <h1>Extensions</h1>
  <p>Manage your browser extensions — 5 built-in, ready to use.</p>
</div>
<div class="ext-grid">
{ext_cards}
</div>
<script>
document.querySelectorAll('.toggle input').forEach(function(cb){{
  cb.addEventListener('change',function(){{
    var idx=this.dataset.idx;
    var card=this.closest('.ext-card');
    var st=card.querySelector('.status');
    if(this.checked){{
      st.textContent='Active';st.className='status active';
    }}else{{
      st.textContent='Inactive';st.className='status';
    }}
    window.location.href='axomai://ext-toggle/'+idx;
  }});
}});
</script>
</body>
</html>"##, ext_cards=ext_cards)
}

pub fn settings_page_html(selected_engine: SearchEngine) -> String {
    let mut engine_options = String::new();
    for engine in SearchEngine::all() {
        let checked = if *engine == selected_engine { "checked" } else { "" };
        let name = engine.name();
        engine_options.push_str(&format!(
            r##"<label class="radio-option">
  <input type="radio" name="engine" value="{name}" {checked}>
  <span class="radio-mark"></span>
  <span class="radio-label">{name}</span>
</label>"##,
            name=name, checked=checked,
        ));
    }

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg:#f8f9fc;--fg:#1a1d2e;--muted:#6b7085;
  --surface:#ffffff;--border:#e4e7f0;
  --accent:#4f46e5;--accent-light:#eef2ff;
  --radius:14px;--shadow:0 2px 12px rgba(0,0,0,.06);
}}
@media(prefers-color-scheme:dark){{
  :root{{
    --bg:#0c0d14;--fg:#e4e6f0;--muted:#7a7f96;
    --surface:#16171f;--border:#252736;
    --accent:#818cf8;--accent-light:rgba(129,140,248,.12);
    --shadow:0 2px 12px rgba(0,0,0,.3);
    color-scheme:dark;
  }}
}}
body{{
  background:var(--bg);color:var(--fg);
  font-family:'Segoe UI',system-ui,-apple-system,sans-serif;
  padding:32px 28px;
  -webkit-user-select:none;user-select:none;
}}
.page-header{{margin-bottom:32px}}
.page-header h1{{font-size:24px;font-weight:700;letter-spacing:-.02em;margin-bottom:4px}}
.page-header p{{color:var(--muted);font-size:14px}}
.section{{
  background:var(--surface);border:1px solid var(--border);
  border-radius:var(--radius);padding:24px;margin-bottom:20px;
}}
.section h2{{font-size:16px;font-weight:600;margin-bottom:4px}}
.section .desc{{color:var(--muted);font-size:13px;margin-bottom:18px}}
.radio-group{{display:flex;flex-direction:column;gap:10px}}
.radio-option{{
  display:flex;align-items:center;gap:12px;
  padding:12px 16px;border-radius:10px;cursor:pointer;
  transition:background .15s;
}}
.radio-option:hover{{background:var(--accent-light)}}
.radio-option input{{display:none}}
.radio-mark{{
  width:20px;height:20px;border-radius:50%;flex-shrink:0;
  border:2px solid var(--border);
  display:flex;align-items:center;justify-content:center;
  transition:border-color .2s;
}}
.radio-mark::after{{
  content:'';width:10px;height:10px;border-radius:50%;
  background:var(--accent);transform:scale(0);transition:transform .2s;
}}
input:checked~.radio-mark{{border-color:var(--accent)}}
input:checked~.radio-mark::after{{transform:scale(1)}}
.radio-label{{font-size:14px;font-weight:500}}
.info-grid{{
  display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));
  gap:14px;
}}
.info-item{{
  padding:16px;border-radius:10px;background:var(--bg);
}}
.info-item .label{{color:var(--muted);font-size:12px;font-weight:500;margin-bottom:4px;text-transform:uppercase;letter-spacing:.04em}}
.info-item .value{{font-size:15px;font-weight:600}}
</style>
</head>
<body>
<div class="page-header">
  <h1>Settings</h1>
  <p>Customize your Axomai Browser experience.</p>
</div>
<div class="section">
  <h2>Search Engine</h2>
  <p class="desc">Choose your default search engine for address bar and home page searches.</p>
  <div class="radio-group">
    {engine_options}
  </div>
</div>
<div class="section">
  <h2>About Axomai Browser</h2>
  <p class="desc">System information and version details.</p>
  <div class="info-grid">
    <div class="info-item">
      <div class="label">Version</div>
      <div class="value">1.0.0</div>
    </div>
    <div class="info-item">
      <div class="label">Engine</div>
      <div class="value">wgpu + WebView2</div>
    </div>
    <div class="info-item">
      <div class="label">Language</div>
      <div class="value">Rust</div>
    </div>
    <div class="info-item">
      <div class="label">Renderer</div>
      <div class="value">GPU (wgpu)</div>
    </div>
  </div>
</div>
<script>
document.querySelectorAll('input[name="engine"]').forEach(function(r){{
  r.addEventListener('change',function(){{
    window.location.href='axomai://set-engine/'+this.value;
  }});
}});
</script>
</body>
</html>"##, engine_options=engine_options)
}
