// Axomai Browser - Interactive UI Handler (Mockup & Native Bridge Ready)

document.addEventListener('DOMContentLoaded', () => {
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
    });
  });

  // 2. New Tab Button
  const btnNewTab = document.getElementById('btnNewTab');
  if (btnNewTab) {
    btnNewTab.addEventListener('click', () => {
      const tabsContainer = document.querySelector('.tabs-container');
      const newTab = document.createElement('div');
      newTab.className = 'tab active';
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

      newTab.addEventListener('click', (e) => {
        if (e.target.closest('.tab-close')) {
          e.stopPropagation();
          newTab.remove();
          return;
        }
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        newTab.classList.add('active');
      });
    });
  }

  // 3. AI Sidebar Toggle
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

  // 4. AI Sub-tabs (Chat, Summarize, Translate, Tools)
  const aiTabs = document.querySelectorAll('.ai-tab');
  aiTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      aiTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
    });
  });

  // 5. News Feed Card Tabs
  const cardTabs = document.querySelectorAll('.card-tab');
  cardTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      cardTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
    });
  });

  // 6. Navigation Buttons Feedback
  const btnReload = document.getElementById('btnReload');
  if (btnReload) {
    btnReload.addEventListener('click', () => {
      btnReload.style.transform = 'rotate(360deg)';
      btnReload.style.transition = 'transform 0.5s ease';
      setTimeout(() => {
        btnReload.style.transform = '';
        btnReload.style.transition = '';
      }, 500);
    });
  }

  // Helper: Navigate via Native Rust Core IPC or fallback
  function navigateTo(url) {
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

  // 7. Omnibox Enter handling
  const omnibox = document.querySelector('.omnibox-input');
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

  // 8. Search Input Enter handling
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
});

// Axomai Core Rust Engine DisplayList HTML5 Canvas Rendering Bridge
window.__axomai_render_display_list = function(displayList) {
  const list = typeof displayList === 'string' ? JSON.parse(displayList) : displayList;
  if (!Array.isArray(list) || list.length === 0) return;

  let canvas = document.getElementById('axomai-engine-canvas');
  if (!canvas) {
    canvas = document.createElement('canvas');
    canvas.id = 'axomai-engine-canvas';
    canvas.style.position = 'fixed';
    canvas.style.top = '88px';
    canvas.style.left = '72px';
    canvas.style.right = '0';
    canvas.style.bottom = '0';
    canvas.style.width = 'calc(100vw - 72px)';
    canvas.style.height = 'calc(100vh - 88px)';
    canvas.style.zIndex = '900';
    canvas.style.backgroundColor = '#ffffff';
    document.body.appendChild(canvas);

    canvas.addEventListener('click', (e) => {
      const rect = canvas.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const clickY = e.clientY - rect.top;
      if (window.ipc) {
        window.ipc.postMessage(`click:${clickX},${clickY}`);
      }
    });
  }

  canvas.width = canvas.clientWidth || 1200;
  canvas.height = canvas.clientHeight || 800;
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  for (const cmd of list) {
    if (cmd.type === 'rect') {
      ctx.fillStyle = cmd.color || '#202124';
      ctx.fillRect(cmd.x1, cmd.y1, cmd.x2 - cmd.x1, cmd.y2 - cmd.y1);
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
      ctx.fillStyle = '#f1f3f4';
      ctx.fillRect(cmd.x, cmd.y, cmd.width, cmd.height);
      ctx.strokeStyle = '#dadce0';
      ctx.strokeRect(cmd.x, cmd.y, cmd.width, cmd.height);
    }
  }
};
