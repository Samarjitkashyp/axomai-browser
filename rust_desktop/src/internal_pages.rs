use crate::types::{Extension, SearchEngine};

pub fn extensions_page_html(extensions: &[Extension]) -> String {
    let mut ext_cards = String::new();
    for (i, ext) in extensions.iter().enumerate() {
        let status_class = if ext.enabled { "active" } else { "" };
        let status_text = if ext.enabled { "Active" } else { "Inactive" };
        let badge = "";
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
    <button class="run" data-idx="{idx}" {run_disabled}>{run_label}</button>
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
            run_disabled=if ext.enabled { "" } else { "disabled" },
            run_label=crate::extensions::ACTION_LABEL.get(i).copied().unwrap_or("Open"),
        ));
    }

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Extensions - Axomai Browser</title>
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
.run{{border:0;border-radius:100px;padding:6px 14px;font-size:12px;font-weight:600;cursor:pointer;background:var(--accent);color:#fff}}
.run:disabled{{opacity:.4;cursor:default}}
</style>
</head>
<body>
<div class="page-header">
  <h1>Extensions</h1>
  <p>Manage your browser extensions — 6 built-in. Switch them on or off here, or use the puzzle icon in the toolbar.</p>
</div>
<div class="ext-grid">
{ext_cards}
</div>
<script>
document.querySelectorAll('.run').forEach(function(b){{
  b.addEventListener('click',function(){{if(!this.disabled)window.location.href='axomai://ext-run/'+this.dataset.idx}});
}});
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

pub fn settings_page_html(selected_engine: SearchEngine, restore_session: bool) -> String {
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
<title>Settings - Axomai Browser</title>
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
.toggle-option{{
  display:flex;align-items:center;gap:14px;cursor:pointer;
  padding:12px 16px;border-radius:10px;transition:background .15s;
}}
.toggle-option:hover{{background:var(--accent-light)}}
.toggle-option input{{display:none}}
.toggle-track{{
  width:44px;height:24px;border-radius:12px;flex-shrink:0;
  background:var(--border);position:relative;transition:background .2s;
}}
.toggle-thumb{{
  position:absolute;top:2px;left:2px;width:20px;height:20px;
  border-radius:50%;background:#fff;transition:transform .2s;
  box-shadow:0 1px 3px rgba(0,0,0,.15);
}}
input:checked~.toggle-track{{background:var(--accent)}}
input:checked~.toggle-track .toggle-thumb{{transform:translateX(20px)}}
.toggle-label{{font-size:14px;font-weight:500}}
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
  <h2>Session Restore</h2>
  <p class="desc">Restore your tabs from the previous session when you open the browser.</p>
  <label class="toggle-option">
    <input type="checkbox" id="restore-toggle" {restore_checked}>
    <span class="toggle-track"><span class="toggle-thumb"></span></span>
    <span class="toggle-label">Restore previous session tabs</span>
  </label>
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
  <a href="axomai://about" style="display:inline-block;margin-top:18px;padding:10px 28px;background:#3b82f6;color:#fff;border-radius:8px;text-decoration:none;font-weight:600;font-size:15px;transition:background 0.2s;">About Us</a>
</div>
<script>
document.querySelectorAll('input[name="engine"]').forEach(function(r){{
  r.addEventListener('change',function(){{
    window.location.href='axomai://set-engine/'+this.value;
  }});
}});
document.getElementById('restore-toggle').addEventListener('change',function(){{
  window.location.href='axomai://set-restore-session/'+(this.checked?'true':'false');
}});
</script>
</body>
</html>"##, engine_options=engine_options,
     restore_checked=if restore_session { "checked" } else { "" })
}
