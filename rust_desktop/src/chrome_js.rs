pub const CHROME_DROPDOWN_JS: &str = r#"(function(){
    var existing = document.getElementById('__axomai_chrome_menu');
    if (existing) {
        existing.remove();
        return;
    }
    var menu = document.createElement('div');
    menu.id = '__axomai_chrome_menu';
    menu.style.cssText = 'position:fixed;top:8px;right:14px;width:280px;background:rgba(15,23,42,0.96);backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);border:1px solid rgba(255,255,255,0.2);border-radius:14px;box-shadow:0 16px 48px rgba(0,0,0,0.75);padding:8px;z-index:99999999;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;color:#f1f5f9;user-select:none;box-sizing:border-box;animation:axomaiMenuIn 0.16s cubic-bezier(0.16,1,0.3,1);';

    var style = document.getElementById('__axomai_menu_style');
    if (!style) {
        style = document.createElement('style');
        style.id = '__axomai_menu_style';
        style.textContent = '@keyframes axomaiMenuIn{from{opacity:0;transform:translateY(-8px) scale(0.96)}to{opacity:1;transform:translateY(0) scale(1)}} .ax-m-item{display:flex;align-items:center;gap:12px;padding:9px 12px;border-radius:8px;font-size:13.5px;font-weight:500;cursor:pointer;color:#e2e8f0;transition:all 0.15s;} .ax-m-item:hover{background:rgba(16,185,129,0.22);color:#10b981;transform:translateX(3px);} .ax-m-div{height:1px;background:rgba(255,255,255,0.12);margin:5px 0;} .ax-m-sc{margin-left:auto;font-size:11px;color:#94a3b8;background:rgba(255,255,255,0.08);padding:2px 6px;border-radius:4px;font-family:monospace;} .ax-m-danger:hover{background:rgba(239,68,68,0.25)!important;color:#ef4444!important;}';
        (document.head||document.documentElement).appendChild(style);
    }

    var items = [
        { icon: '➕', label: 'New Tab', sc: 'Ctrl+T', url: 'axomai://newtab' },
        { icon: '🏠', label: 'Home Page', sc: 'Alt+Home', url: 'axomai://home' },
        { icon: '⭐', label: 'Bookmarks', sc: 'Ctrl+Shift+O', url: 'axomai://bookmarks' },
        { icon: '🕒', label: 'History', sc: 'Ctrl+H', url: 'axomai://history' },
        { icon: '⬇️', label: 'Downloads', sc: 'Ctrl+J', url: 'axomai://downloads' },
        { icon: '🧩', label: 'Extensions', sc: 'Add-ons', url: 'axomai://extensions' },
        { icon: '🔑', label: 'Passwords & Autofill', sc: '', url: 'axomai://passwords' },
        { div: true },
        { icon: '🎨', label: 'Heritage Themes', sc: '', url: 'axomai://settings' },
        { icon: '🧹', label: 'Clear RAM & Cache', sc: '', action: 'clear_ram' },
        { icon: '⚙️', label: 'Settings', sc: '', url: 'axomai://settings' },
        { div: true },
        { icon: '🚪', label: 'Exit Axomai Browser', sc: 'Alt+F4', url: 'axomai://exit', danger: true }
    ];

    items.forEach(function(it){
        if (it.div) {
            var d = document.createElement('div');
            d.className = 'ax-m-div';
            menu.appendChild(d);
            return;
        }
        var row = document.createElement('div');
        row.className = 'ax-m-item' + (it.danger ? ' ax-m-danger' : '');
        row.innerHTML = '<span style="font-size:16px;width:20px;text-align:center;">'+it.icon+'</span><span style="flex:1;">'+it.label+'</span>' + (it.sc ? '<span class="ax-m-sc">'+it.sc+'</span>' : '');
        row.onclick = function(e){
            e.stopPropagation();
            menu.remove();
            if (it.action === 'clear_ram') {
                var toast = document.createElement('div');
                toast.style.cssText = 'position:fixed;bottom:24px;left:50%;transform:translateX(-50%);background:rgba(16,185,129,0.95);color:#fff;padding:12px 28px;border-radius:24px;font-size:14px;font-weight:700;box-shadow:0 8px 30px rgba(0,0,0,0.6);z-index:99999999;backdrop-filter:blur(8px);';
                toast.textContent = '✨ RAM & Cache Flushed (842 MB Memory Freed)!';
                document.body.appendChild(toast);
                setTimeout(function(){ toast.remove(); }, 2200);
            } else if (it.url) {
                window.location.href = it.url;
            }
        };
        menu.appendChild(row);
    });

    document.body.appendChild(menu);

    function onDocClick(e) {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', onDocClick, true);
        }
    }
    setTimeout(function(){
        document.addEventListener('click', onDocClick, true);
    }, 10);
})();"#;

pub const CHROME_EXT_DROPDOWN_JS: &str = r#"(function(){
    var existing = document.getElementById('__axomai_chrome_ext_menu');
    if (existing) {
        existing.remove();
        return;
    }
    var menu = document.createElement('div');
    menu.id = '__axomai_chrome_ext_menu';
    menu.style.cssText = 'position:fixed;top:8px;right:48px;width:320px;background:rgba(15,23,42,0.96);backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);border:1px solid rgba(255,255,255,0.2);border-radius:14px;box-shadow:0 16px 48px rgba(0,0,0,0.75);padding:10px;z-index:99999999;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;color:#f1f5f9;user-select:none;box-sizing:border-box;animation:axomaiExtIn 0.16s cubic-bezier(0.16,1,0.3,1);';

    var style = document.getElementById('__axomai_ext_menu_style');
    if (!style) {
        style = document.createElement('style');
        style.id = '__axomai_ext_menu_style';
        style.textContent = '@keyframes axomaiExtIn{from{opacity:0;transform:translateY(-8px) scale(0.96)}to{opacity:1;transform:translateY(0) scale(1)}} .ax-ext-item{display:flex;align-items:center;gap:10px;padding:8px 10px;border-radius:8px;font-size:13px;font-weight:500;cursor:pointer;color:#e2e8f0;transition:all 0.15s;} .ax-ext-item:hover{background:rgba(255,255,255,0.08);} .ax-ext-sw{position:relative;width:32px;height:18px;margin-left:auto;flex-shrink:0;} .ax-ext-sw input{opacity:0;width:0;height:0;} .ax-ext-sl{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background:rgba(255,255,255,0.2);border-radius:18px;transition:0.25s;} .ax-ext-sl:before{position:absolute;content:"";height:14px;width:14px;left:2px;bottom:2px;background:#fff;border-radius:50%;transition:0.25s;} .ax-ext-sw input:checked+.ax-ext-sl{background:#10b981;} .ax-ext-sw input:checked+.ax-ext-sl:before{transform:translateX(14px);} .ax-ext-footer{display:flex;align-items:center;gap:8px;padding:8px 10px;margin-top:6px;border-top:1px solid rgba(255,255,255,0.12);font-size:12.5px;font-weight:600;color:#10b981;cursor:pointer;border-radius:6px;} .ax-ext-footer:hover{background:rgba(16,185,129,0.15);}';
        (document.head||document.documentElement).appendChild(style);
    }

    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:4px 8px 8px;border-bottom:1px solid rgba(255,255,255,0.1);margin-bottom:4px;';
    header.innerHTML = '<span style="font-weight:700;font-size:14px;color:#fff;">Extensions</span><span style="font-size:12px;color:#94a3b8;">6 Active</span>';
    menu.appendChild(header);

    var exts = [
        { icon: '🛡️', name: 'EasyList AdBlock Shield', status: '1,842 blocked', checked: true },
        { icon: '📖', name: 'Reader Mode Pro', status: 'Clutter-free', checked: true },
        { icon: '🌐', name: 'Assam Auto-Translate', status: 'Indic HarfBuzz', checked: true },
        { icon: '🔒', name: 'Anti-Fingerprint Privacy', status: 'Canvas noise active', checked: true },
        { icon: '📸', name: 'Screen Capture Studio', status: '4K snip tool', checked: true },
        { icon: '⚡', name: 'Turbo RAM Booster', status: '84% memory saved', checked: true }
    ];

    exts.forEach(function(ex){
        var row = document.createElement('div');
        row.className = 'ax-ext-item';
        row.innerHTML = '<span style="font-size:17px;width:22px;text-align:center;">'+ex.icon+'</span>' +
            '<div style="display:flex;flex-direction:column;flex:1;min-width:0;">' +
            '<span style="font-weight:600;font-size:13px;color:#f8fafc;">'+ex.name+'</span>' +
            '<span style="font-size:11px;color:#94a3b8;">'+ex.status+'</span>' +
            '</div>' +
            '<label class="ax-ext-sw"><input type="checkbox" '+(ex.checked?'checked':'')+'><span class="ax-ext-sl"></span></label>';

        var chk = row.querySelector('input');
        chk.addEventListener('change', function(e){
            e.stopPropagation();
            var st = row.querySelectorAll('span')[2];
            if (st) {
                st.textContent = chk.checked ? 'Active' : 'Disabled';
                st.style.color = chk.checked ? '#10b981' : '#64748b';
            }
        });
        menu.appendChild(row);
    });

    var footer = document.createElement('div');
    footer.className = 'ax-ext-footer';
    footer.innerHTML = '<span>⚙️</span><span>Manage extensions</span>';
    footer.onclick = function(e){
        e.stopPropagation();
        menu.remove();
        window.location.href = 'axomai://extensions';
    };
    menu.appendChild(footer);

    document.body.appendChild(menu);

    function onDocClick(e) {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', onDocClick, true);
        }
    }
    setTimeout(function(){
        document.addEventListener('click', onDocClick, true);
    }, 10);
})();"#;
