// Axomai Browser - Interactive UI Handler & Native Engine Bridge

const imageCache = new Map();
let lastDisplayList = null;

document.addEventListener('DOMContentLoaded', () => {
  const newTabView = document.getElementById('newTabView');
  const engineViewport = document.getElementById('engineViewport');
  const omnibox = document.querySelector('.omnibox-input');
  const btnBack = document.getElementById('btnBack');
  const btnForward = document.getElementById('btnForward');
  const btnReload = document.getElementById('btnReload');

  function showNewTab() {
    if (newTabView) newTabView.style.display = 'flex';
    if (engineViewport) engineViewport.style.display = 'none';
    if (omnibox) omnibox.value = '';
    const activeTabTitle = document.querySelector('.tab.active .tab-title');
    if (activeTabTitle) activeTabTitle.textContent = 'New Tab';
  }

  function showEngineViewport() {
    if (newTabView) newTabView.style.display = 'none';
    if (engineViewport) engineViewport.style.display = 'flex';
  }

  // 1. Tab Switching
  const tabs = document.querySelectorAll('.tab');
  tabs.forEach(tab => {
    tab.addEventListener('click', (e) => {
      if (e.target.closest('.tab-close')) {
        e.stopPropagation();
        if (tabs.length > 1) {
          tab.remove();
        }
        return;
      }
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const tabType = tab.getAttribute('data-tab');
      if (tabType === 'new-tab') {
        showNewTab();
      } else if (tabType === 'assam') {
        navigateTo('https://tourism.assam.gov.in');
      } else if (tabType === 'youtube') {
        navigateTo('https://youtube.com');
      }
    });
  });

  // 2. New Tab Button
  const btnNewTab = document.getElementById('btnNewTab');
  if (btnNewTab) {
    btnNewTab.addEventListener('click', () => {
      const tabsContainer = document.querySelector('.tabs-container');
      const newTab = document.createElement('div');
      newTab.className = 'tab active';
      newTab.setAttribute('data-tab', 'new-tab');
      newTab.innerHTML = `
        <div class="tab-icon">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
            <path d="M19 4H5c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H5V8h14v10z"/>
          </svg>
        </div>
        <span class="tab-title">New Tab</span>
        <button class="tab-close" title="Close Tab">
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      `;
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      tabsContainer.insertBefore(newTab, btnNewTab);
      showNewTab();

      newTab.addEventListener('click', (e) => {
        if (e.target.closest('.tab-close')) {
          e.stopPropagation();
          newTab.remove();
          return;
        }
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        newTab.classList.add('active');
        showNewTab();
      });
    });
  }

  // 3. Left Sidebar Home Button
  const navHome = document.querySelector('.nav-item.active') || document.querySelector('.nav-item');
  if (navHome) {
    navHome.addEventListener('click', () => {
      showNewTab();
    });
  }

  // 4. Navigation Controls (Back, Forward, Reload)
  if (btnBack) {
    btnBack.addEventListener('click', () => {
      if (window.ipc) {
        window.ipc.postMessage('back');
      }
    });
  }

  if (btnForward) {
    btnForward.addEventListener('click', () => {
      if (window.ipc) {
        window.ipc.postMessage('forward');
      }
    });
  }

  if (btnReload) {
    btnReload.addEventListener('click', () => {
      btnReload.style.transform = 'rotate(360deg)';
      btnReload.style.transition = 'transform 0.5s ease';
      setTimeout(() => {
        btnReload.style.transform = '';
        btnReload.style.transition = '';
      }, 500);
      if (window.ipc) {
        window.ipc.postMessage('reload');
      } else if (omnibox && omnibox.value.trim()) {
        navigateTo(omnibox.value.trim());
      }
    });
  }

  // 5. AI Sidebar Toggle
  const btnToggleAi = document.getElementById('btnToggleAi');
  const btnCloseAi = document.getElementById('btnCloseAi');
  const aiSidebar = document.getElementById('aiSidebar');

  if (btnToggleAi && aiSidebar) {
    btnToggleAi.addEventListener('click', () => {
      aiSidebar.style.display = aiSidebar.style.display === 'none' ? 'flex' : 'none';
    });
  }

  if (btnCloseAi && aiSidebar) {
    btnCloseAi.addEventListener('click', () => {
      aiSidebar.style.display = 'none';
    });
  }

  // 6. AI Sub-tabs (Chat, Summarize, Translate, Tools)
  const aiTabs = document.querySelectorAll('.ai-tab');
  aiTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      aiTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
    });
  });

  // 7. News Feed Card Tabs
  const cardTabs = document.querySelectorAll('.card-tab');
  cardTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      cardTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
    });
  });

  // Helper: Navigate via Native Rust Core IPC or fallback
  function navigateTo(url) {
    if (omnibox) {
      omnibox.value = url;
    }
    showEngineViewport();
    if (window.ipc) {
      window.ipc.postMessage('navigate:' + url);
    } else {
      if (url.startsWith('http://') || url.startsWith('https://')) {
        window.location.href = url;
      } else {
        window.location.href = `https://www.google.com/search?q=${encodeURIComponent(url)}`;
      }
    }
  }

  // 8. Omnibox Enter handling
  if (omnibox) {
    omnibox.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const query = omnibox.value.trim();
        if (query) {
          navigateTo(query);
        }
      }
    });
  }

  // 9. Search Input Enter handling
  const searchInput = document.querySelector('.search-input');
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const query = searchInput.value.trim();
        if (query) {
          navigateTo(`https://www.google.com/search?q=${encodeURIComponent(query)}`);
        }
      }
    });
  }

  // 10. Shortcut items click
  document.querySelectorAll('.shortcut-item').forEach(item => {
    item.addEventListener('click', (e) => {
      const href = item.getAttribute('href');
      if (href && href.startsWith('http')) {
        e.preventDefault();
        navigateTo(href);
      }
    });
  });
});

// Axomai Core Rust Engine DisplayList HTML5 Canvas Rendering Bridge
window.__axomai_render_display_list = function(displayList) {
  const list = typeof displayList === 'string' ? JSON.parse(displayList) : displayList;
  if (!Array.isArray(list)) return;

  lastDisplayList = list;

  const engineViewport = document.getElementById('engineViewport');
  const newTabView = document.getElementById('newTabView');
  let canvas = document.getElementById('axomai-engine-canvas');

  if (list.length === 0) {
    if (newTabView) newTabView.style.display = 'flex';
    if (engineViewport) engineViewport.style.display = 'none';
    return;
  }

  if (newTabView) newTabView.style.display = 'none';
  if (engineViewport) engineViewport.style.display = 'flex';

  if (!canvas) {
    canvas = document.createElement('canvas');
    canvas.id = 'axomai-engine-canvas';
    if (engineViewport) {
      engineViewport.appendChild(canvas);
    }
  }

  if (!canvas._clickBound) {
    canvas.addEventListener('mousedown', () => {
      canvas.focus();
    });

    canvas.addEventListener('click', (e) => {
      canvas.focus();
      const rect = canvas.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const clickY = e.clientY - rect.top;
      if (window.ipc) {
        window.ipc.postMessage(`click:${clickX},${clickY}`);
      }
    });

    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const clickY = e.clientY - rect.top;
      if (window.ipc) {
        window.ipc.postMessage(`wheel:${e.deltaY},${e.deltaX || 0},${clickX},${clickY}`);
      }
    }, { passive: false });

    canvas.tabIndex = 0; // Make canvas focusable for keyboard events
    canvas._clickBound = true;
  }

  // Global key listener for native page inputs
  if (!window._axomaiKeyBound) {
    window.addEventListener('keydown', (e) => {
      const active = document.activeElement;
      if (active && (active.classList.contains('omnibox-input') || active.classList.contains('search-input') || active.classList.contains('ai-text-input'))) {
        return; // Don't intercept browser chrome inputs
      }
      const isBrowserShortcut = (e.ctrlKey || e.metaKey || e.altKey) || e.key === 'F5' || e.key === 'F12' || e.key === 'F11';
      if (engineViewport && engineViewport.style.display !== 'none') {
        if (!isBrowserShortcut) {
          if (e.key === 'Backspace' || e.key === 'Enter' || e.key === 'Tab' || e.key === ' ' || e.key.length === 1 || e.key.startsWith('Arrow')) {
            e.preventDefault();
          }
        }
        if (window.ipc) {
          window.ipc.postMessage(`key:${e.key}`);
        }
      }
    });
    window._axomaiKeyBound = true;
  }

  const containerW = (engineViewport && engineViewport.clientWidth) || 1200;
  const containerH = (engineViewport && engineViewport.clientHeight) || 800;
  canvas.width = containerW;
  canvas.height = containerH;

  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  for (const cmd of list) {
    if (cmd.type === 'pushOpacity') {
      ctx.save();
      ctx.globalAlpha = (ctx.globalAlpha || 1.0) * (cmd.opacity !== undefined ? cmd.opacity : 1.0);
    } else if (cmd.type === 'popOpacity') {
      ctx.restore();
    } else if (cmd.type === 'pushClip') {
      ctx.save();
      ctx.beginPath();
      const r = cmd.borderRadius || 0;
      if (r > 0 && typeof ctx.roundRect === 'function') {
        ctx.roundRect(cmd.x, cmd.y, cmd.width, cmd.height, r);
      } else {
        ctx.rect(cmd.x, cmd.y, cmd.width, cmd.height);
      }
      ctx.clip();
    } else if (cmd.type === 'popClip') {
      ctx.restore();
    } else if (cmd.type === 'boxShadow') {
      ctx.save();
      ctx.shadowOffsetX = cmd.offsetX || 0;
      ctx.shadowOffsetY = cmd.offsetY || 0;
      ctx.shadowBlur = cmd.blur || 0;
      ctx.shadowColor = cmd.color || 'rgba(0,0,0,0.2)';
      ctx.fillStyle = cmd.color || 'rgba(0,0,0,0.2)';
      const r = cmd.borderRadius || 0;
      if (r > 0 && typeof ctx.roundRect === 'function') {
        ctx.beginPath();
        ctx.roundRect(cmd.x, cmd.y, cmd.width, cmd.height, r);
        ctx.fill();
      } else {
        ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
      }
      ctx.restore();
    } else if (cmd.type === 'rect') {
      ctx.fillStyle = cmd.color || '#202124';
      const w = cmd.x2 - cmd.x1;
      const h = cmd.y2 - cmd.y1;
      const r = cmd.borderRadius || 0;
      if (r > 0 && typeof ctx.roundRect === 'function') {
        ctx.beginPath();
        ctx.roundRect(cmd.x1, cmd.y1, w, h, r);
        ctx.fill();
      } else {
        ctx.fillRect(cmd.x1, cmd.y1, w, h);
      }
    } else if (cmd.type === 'text') {
      ctx.fillStyle = cmd.color || '#202124';
      ctx.font = `${cmd.fontWeight || 'normal'} ${cmd.fontStyle || 'normal'} ${cmd.fontSize || 14}px 'Segoe UI', sans-serif`;
      ctx.fillText(cmd.text || '', cmd.x, cmd.y + (cmd.fontSize || 14));
    } else if (cmd.type === 'input') {
      ctx.fillStyle = cmd.isFocused ? '#ffffff' : '#f8f9fa';
      ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
      ctx.strokeStyle = cmd.isFocused ? '#1a73e8' : '#dadce0';
      ctx.lineWidth = cmd.isFocused ? 2 : 1;
      ctx.strokeRect(cmd.x, cmd.y, cmd.width, cmd.height);
      ctx.fillStyle = '#202124';
      ctx.font = "14px 'Segoe UI', sans-serif";
      const val = cmd.value || cmd.placeholder || '';
      ctx.fillText(val + (cmd.isFocused ? '|' : ''), cmd.x + 10, cmd.y + 22);
    } else if (cmd.type === 'button') {
      ctx.fillStyle = '#1a73e8';
      ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
      ctx.fillStyle = '#ffffff';
      ctx.font = "bold 13px 'Segoe UI', sans-serif";
      ctx.fillText(cmd.label || '', cmd.x + 12, cmd.y + 20);
    } else if (cmd.type === 'image') {
      if (cmd.src && cmd.src.startsWith('data:')) {
        let cached = imageCache.get(cmd.src);
        if (!cached) {
          cached = new Image();
          cached.src = cmd.src;
          cached.onload = () => {
            if (lastDisplayList) {
              window.__axomai_render_display_list(lastDisplayList);
            }
          };
          imageCache.set(cmd.src, cached);
        }
        if (cached.complete && cached.naturalWidth > 0) {
          ctx.drawImage(cached, cmd.x, cmd.y, cmd.width, cmd.height);
        } else {
          ctx.fillStyle = '#f1f3f4';
          ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
        }
      } else {
        ctx.fillStyle = '#f1f3f4';
        ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
        ctx.strokeStyle = '#dadce0';
        ctx.strokeRect(cmd.x, cmd.y, cmd.width, cmd.height);
      }
    }
  }
};

// Navigation Synchronization Bridge from Native Engine
window.__axomai_sync_navigation = function(url, title, canBack, canForward) {
  const omnibox = document.querySelector('.omnibox-input');
  if (omnibox && document.activeElement !== omnibox) {
    omnibox.value = url || '';
  }

  const activeTabTitle = document.querySelector('.tab.active .tab-title');
  if (activeTabTitle) {
    activeTabTitle.textContent = title || url || 'New Tab';
  }

  if (title) {
    document.title = `${title} - Axomai Browser`;
  }

  const btnBack = document.getElementById('btnBack');
  if (btnBack) {
    btnBack.style.opacity = canBack ? '1.0' : '0.4';
    btnBack.style.cursor = canBack ? 'pointer' : 'default';
  }

  const btnForward = document.getElementById('btnForward');
  if (btnForward) {
    btnForward.style.opacity = canForward ? '1.0' : '0.4';
    btnForward.style.cursor = canForward ? 'pointer' : 'default';
  }
};


