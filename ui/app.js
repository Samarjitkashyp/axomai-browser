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

  // 7. Omnibox Enter handling
  const omnibox = document.querySelector('.omnibox-input');
  if (omnibox) {
    omnibox.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const query = omnibox.value.trim();
        if (query) {
          if (query.startsWith('http://') || query.startsWith('https://')) {
            window.location.href = query;
          } else {
            window.location.href = `https://www.google.com/search?q=${encodeURIComponent(query)}`;
          }
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
          window.location.href = `https://www.google.com/search?q=${encodeURIComponent(query)}`;
        }
      }
    });
  }
});
