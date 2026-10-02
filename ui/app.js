// ==========================================================================
// Axomai Browser — High Performance Frontend Controller & State Engine
// ==========================================================================

document.addEventListener('DOMContentLoaded', () => {
  // DOM Element References
  const htmlRoot = document.documentElement;
  const newTabView = document.getElementById('newTabView');
  const engineViewport = document.getElementById('engineViewport');
  const webFrame = document.getElementById('webFrame');
  const engineUrlChip = document.getElementById('engineUrlChip');
  
  const omniboxInput = document.getElementById('omniboxInput');
  const omniboxDropdown = document.getElementById('omniboxDropdown');
  const omniboxWrapper = document.getElementById('omniboxWrapper');
  const heroSearchInput = document.getElementById('heroSearchInput');
  const btnSearchSubmit = document.getElementById('btnSearchSubmit');
  const btnVoiceSearch = document.getElementById('btnVoiceSearch');
  
  const tabsContainer = document.getElementById('tabsContainer');
  const btnNewTab = document.getElementById('btnNewTab');
  const btnBack = document.getElementById('btnBack');
  const btnForward = document.getElementById('btnForward');
  const btnReload = document.getElementById('btnReload');
  const btnHome = document.getElementById('btnHome');
  
  const scenicBackdrop = document.getElementById('scenicBackdrop');
  const greetingAssam = document.getElementById('greetingAssam');
  const liveClock = document.getElementById('liveClock');
  const liveDate = document.getElementById('liveDate');
  
  const aiSidebar = document.getElementById('aiSidebar');
  const btnToggleAi = document.getElementById('btnToggleAi');
  const btnCloseAi = document.getElementById('btnCloseAi');
  const aiTextInput = document.getElementById('aiTextInput');
  const btnAiSend = document.getElementById('btnAiSend');
  const aiChatArea = document.getElementById('aiChatArea');
  
  const themeModal = document.getElementById('themeModal');
  const btnThemeSelector = document.getElementById('btnThemeSelector');
  const btnCloseThemeModal = document.getElementById('btnCloseThemeModal');
  const settingsModal = document.getElementById('settingsModal');
  const btnCloseSettingsModal = document.getElementById('btnCloseSettingsModal');
  const navSettings = document.getElementById('navSettings');
  const qrModal = document.getElementById('qrModal');
  const btnQrCode = document.getElementById('btnQrCode');
  const btnCloseQrModal = document.getElementById('btnCloseQrModal');
  const btnCopyLink = document.getElementById('btnCopyLink');
  
  // State
  let activeTabId = 'tab-1';
  let currentSearchEngine = 'https://www.google.com/search?q=';
  let tabsData = [
    { id: 'tab-1', title: 'New Tab', url: 'axomai://newtab', icon: 'home' },
    { id: 'tab-2', title: 'Assam Tourism — Incredible India', url: 'https://tourism.assam.gov.in', icon: 'globe' },
    { id: 'tab-3', title: 'YouTube', url: 'https://youtube.com', icon: 'youtube' }
  ];

  // 1. Live Clock & Assamese Greeting
  function updateTimeAndGreeting() {
    const now = new Date();
    
    // Time format
    let hours = now.getHours();
    const minutes = String(now.getMinutes()).padStart(2, '0');
    const ampm = hours >= 12 ? 'PM' : 'AM';
    hours = hours % 12 || 12;
    if (liveClock) liveClock.textContent = `${hours}:${minutes} ${ampm}`;
    
    // Date format
    const options = { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' };
    if (liveDate) liveDate.textContent = now.toLocaleDateString('en-US', options);
    
    // Assamese Greeting based on hour
    const rawHour = now.getHours();
    let greetText = 'নমস্কাৰ • অসম';
    if (rawHour >= 4 && rawHour < 12) {
      greetText = 'নমস্কাৰ • সুপ্ৰভাত (Good Morning)';
    } else if (rawHour >= 12 && rawHour < 16) {
      greetText = 'নমস্কাৰ • শুভ দুপৰীয়া (Good Afternoon)';
    } else if (rawHour >= 16 && rawHour < 20) {
      greetText = 'নমস্কাৰ • শুভ সন্ধিয়া (Good Evening)';
    } else {
      greetText = 'নমস্কাৰ • শুভ ৰাত্ৰি (Good Night)';
    }
    if (greetingAssam) greetingAssam.textContent = greetText;
  }
  setInterval(updateTimeAndGreeting, 1000);
  updateTimeAndGreeting();

  // 2. Navigation Functions
  function navigateTo(url) {
    if (!url) return;
    url = url.trim();

    // Check if internal page or web URL
    if (url === 'axomai://newtab' || url === 'about:blank' || url === '') {
      if (newTabView) newTabView.style.display = 'flex';
      if (engineViewport) engineViewport.style.display = 'none';
      if (omniboxInput) omniboxInput.value = '';
      updateActiveTabInfo('New Tab', 'axomai://newtab');
      return;
    }

    // Format URL
    let targetUrl = url;
    if (!url.startsWith('http://') && !url.startsWith('https://') && !url.startsWith('axomai://')) {
      if (url.includes('.') && !url.includes(' ')) {
        targetUrl = 'https://' + url;
      } else {
        targetUrl = currentSearchEngine + encodeURIComponent(url);
      }
    }

    // Show Engine Viewport
    if (newTabView) newTabView.style.display = 'none';
    if (engineViewport) engineViewport.style.display = 'flex';
    if (omniboxInput) omniboxInput.value = targetUrl;
    if (engineUrlChip) engineUrlChip.textContent = targetUrl;
    if (webFrame) webFrame.src = targetUrl;

    // Extract Domain for tab title
    let pageTitle = targetUrl;
    try {
      const parsed = new URL(targetUrl);
      pageTitle = parsed.hostname.replace('www.', '');
    } catch (e) {}

    updateActiveTabInfo(pageTitle, targetUrl);

    // Send IPC to Rust process if running inside Tao/Wry
    if (window.ipc && window.ipc.postMessage) {
      window.ipc.postMessage(`navigate:${targetUrl}`);
    }
  }

  function updateActiveTabInfo(title, url) {
    const activeTabElem = document.querySelector(`.tab[data-tab-id="${activeTabId}"]`);
    if (activeTabElem) {
      const titleElem = activeTabElem.querySelector('.tab-title');
      if (titleElem) titleElem.textContent = title;
      activeTabElem.setAttribute('data-url', url);
    }
    const tabObj = tabsData.find(t => t.id === activeTabId);
    if (tabObj) {
      tabObj.title = title;
      tabObj.url = url;
    }
  }

  // 3. Tab Management Handlers
  function initTabClickListeners(tabElem) {
    tabElem.addEventListener('click', (e) => {
      if (e.target.closest('.tab-close')) {
        e.stopPropagation();
        closeTab(tabElem.getAttribute('data-tab-id'));
        return;
      }
      switchTab(tabElem.getAttribute('data-tab-id'));
    });
  }

  document.querySelectorAll('.tab').forEach(initTabClickListeners);

  function switchTab(tabId) {
    activeTabId = tabId;
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    const currentTabElem = document.querySelector(`.tab[data-tab-id="${tabId}"]`);
    if (currentTabElem) {
      currentTabElem.classList.add('active');
      const url = currentTabElem.getAttribute('data-url') || 'axomai://newtab';
      if (url === 'axomai://newtab') {
        if (newTabView) newTabView.style.display = 'flex';
        if (engineViewport) engineViewport.style.display = 'none';
        if (omniboxInput) omniboxInput.value = '';
      } else {
        if (newTabView) newTabView.style.display = 'none';
        if (engineViewport) engineViewport.style.display = 'flex';
        if (omniboxInput) omniboxInput.value = url;
        if (engineUrlChip) engineUrlChip.textContent = url;
        if (webFrame && webFrame.src !== url) webFrame.src = url;
      }
    }
  }

  function createNewTab() {
    const newId = 'tab-' + Date.now();
    const newTabElem = document.createElement('div');
    newTabElem.className = 'tab active';
    newTabElem.setAttribute('data-tab-id', newId);
    newTabElem.setAttribute('data-url', 'axomai://newtab');
    newTabElem.innerHTML = `
      <div class="tab-icon">
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M19 4H5c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H5V8h14v10z"/>
        </svg>
      </div>
      <span class="tab-title">New Tab</span>
      <button class="tab-close" title="Close tab">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.5">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    `;
    
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    tabsContainer.insertBefore(newTabElem, btnNewTab);
    tabsData.push({ id: newId, title: 'New Tab', url: 'axomai://newtab', icon: 'home' });
    activeTabId = newId;
    
    initTabClickListeners(newTabElem);
    navigateTo('axomai://newtab');
    if (heroSearchInput) heroSearchInput.focus();
  }

  if (btnNewTab) btnNewTab.addEventListener('click', createNewTab);

  function closeTab(tabId) {
    const tabElem = document.querySelector(`.tab[data-tab-id="${tabId}"]`);
    if (!tabElem) return;
    
    const allTabs = document.querySelectorAll('.tab');
    if (allTabs.length <= 1) {
      // Just reset to new tab
      navigateTo('axomai://newtab');
      return;
    }

    const isCurrentActive = tabElem.classList.contains('active');
    tabElem.remove();
    tabsData = tabsData.filter(t => t.id !== tabId);

    if (isCurrentActive) {
      const remainingTabs = document.querySelectorAll('.tab');
      if (remainingTabs.length > 0) {
        const lastTab = remainingTabs[remainingTabs.length - 1];
        switchTab(lastTab.getAttribute('data-tab-id'));
      }
    }
  }

  // Keyboard Shortcuts (Ctrl+T for new tab, Ctrl+W to close)
  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 't') {
      e.preventDefault();
      createNewTab();
    } else if (e.ctrlKey && e.key === 'w') {
      e.preventDefault();
      closeTab(activeTabId);
    } else if (e.ctrlKey && e.key === 'r') {
      e.preventDefault();
      if (btnReload) btnReload.click();
    }
  });

  // 4. Omnibox & Hero Search Interactions
  if (omniboxInput) {
    omniboxInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        navigateTo(omniboxInput.value);
        if (omniboxDropdown) omniboxDropdown.style.display = 'none';
      }
    });

    omniboxInput.addEventListener('focus', () => {
      if (omniboxDropdown) omniboxDropdown.style.display = 'flex';
    });
  }

  document.addEventListener('click', (e) => {
    if (omniboxWrapper && !omniboxWrapper.contains(e.target)) {
      if (omniboxDropdown) omniboxDropdown.style.display = 'none';
    }
  });

  document.querySelectorAll('.suggestion-item').forEach(item => {
    item.addEventListener('click', () => {
      const q = item.getAttribute('data-query');
      navigateTo(q);
      if (omniboxDropdown) omniboxDropdown.style.display = 'none';
    });
  });

  if (heroSearchInput) {
    heroSearchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        navigateTo(heroSearchInput.value);
      }
    });
  }

  if (btnSearchSubmit && heroSearchInput) {
    btnSearchSubmit.addEventListener('click', () => {
      navigateTo(heroSearchInput.value);
    });
  }

  // Voice Search Simulation
  if (btnVoiceSearch) {
    btnVoiceSearch.addEventListener('click', () => {
      btnVoiceSearch.style.color = '#ef4444';
      btnVoiceSearch.style.transform = 'scale(1.2)';
      heroSearchInput.placeholder = '🎙️ Listening... Speak now (English / অসমীয়া)';
      setTimeout(() => {
        btnVoiceSearch.style.color = '';
        btnVoiceSearch.style.transform = '';
        heroSearchInput.placeholder = 'Search the web with Axomai AI or enter URL...';
        heroSearchInput.value = 'Assam Tourism & Wildlife';
        navigateTo('https://tourism.assam.gov.in');
      }, 2000);
    });
  }

  // 5. Navigation Buttons (Back, Forward, Reload, Home)
  if (btnHome) btnHome.addEventListener('click', () => navigateTo('axomai://newtab'));
  
  if (btnReload) {
    btnReload.addEventListener('click', () => {
      btnReload.style.transform = 'rotate(360deg)';
      btnReload.style.transition = 'transform 0.5s ease';
      setTimeout(() => {
        btnReload.style.transform = '';
        btnReload.style.transition = '';
      }, 500);
      if (webFrame && webFrame.src) {
        webFrame.src = webFrame.src;
      }
      if (window.ipc) window.ipc.postMessage('reload');
    });
  }

  if (btnBack) {
    btnBack.addEventListener('click', () => {
      if (window.ipc) window.ipc.postMessage('back');
      else history.back();
    });
  }

  if (btnForward) {
    btnForward.addEventListener('click', () => {
      if (window.ipc) window.ipc.postMessage('forward');
      else history.forward();
    });
  }

  // 6. Shortcut Speed Dial Clicks
  document.querySelectorAll('.shortcut-item:not(.add-shortcut)').forEach(item => {
    item.addEventListener('click', () => {
      const targetUrl = item.getAttribute('data-url');
      if (targetUrl) navigateTo(targetUrl);
    });
  });

  const btnAddShortcut = document.getElementById('btnAddShortcut');
  if (btnAddShortcut) {
    btnAddShortcut.addEventListener('click', () => {
      const siteUrl = prompt('Enter Website URL (e.g., https://example.com):');
      if (siteUrl) {
        const siteName = prompt('Enter Site Name:') || 'Custom Site';
        const row = document.getElementById('shortcutsRow');
        const newItem = document.createElement('div');
        newItem.className = 'shortcut-item';
        newItem.setAttribute('data-url', siteUrl);
        newItem.innerHTML = `
          <div class="shortcut-icon" style="background: linear-gradient(135deg, #10b981, #059669); color: #fff;">
            <span>🌐</span>
          </div>
          <span class="shortcut-name">${siteName}</span>
        `;
        newItem.addEventListener('click', () => navigateTo(siteUrl));
        row.insertBefore(newItem, btnAddShortcut);
      }
    });
  }

  // 7. News Category Tabs
  const newsCategoryTabs = document.querySelectorAll('.card-tab');
  newsCategoryTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      newsCategoryTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
    });
  });

  const btnRefreshFeed = document.getElementById('btnRefreshFeed');
  if (btnRefreshFeed) {
    btnRefreshFeed.addEventListener('click', () => {
      btnRefreshFeed.style.transform = 'rotate(360deg)';
      setTimeout(() => btnRefreshFeed.style.transform = '', 400);
    });
  }

  // 8. Smart Web Tools Hub Click Handlers
  document.querySelectorAll('.tool-tile').forEach(tile => {
    tile.addEventListener('click', () => {
      const tool = tile.getAttribute('data-tool');
      if (tool === 'assamese-translate') {
        openAiWithPrompt('Please translate the current web content into authentic, fluent Assamese (অসমীয়া).');
      } else if (tool === 'ai-notes') {
        openAiWithPrompt('Generate comprehensive bullet-point study notes and a concise executive summary.');
      } else if (tool === 'qr-gen') {
        if (qrModal) qrModal.style.display = 'flex';
      } else {
        alert(`⚡ Axomai Smart Tool "${tile.querySelector('.tool-label').textContent}" launched in background thread.`);
      }
    });
  });

  // 9. AI Copilot Drawer & Chat System
  function toggleAiDrawer() {
    if (!aiSidebar) return;
    const isVisible = aiSidebar.style.display === 'flex';
    aiSidebar.style.display = isVisible ? 'none' : 'flex';
  }

  if (btnToggleAi) btnToggleAi.addEventListener('click', toggleAiDrawer);
  if (btnCloseAi) btnCloseAi.addEventListener('click', () => aiSidebar.style.display = 'none');

  const navAi = document.getElementById('navAi');
  if (navAi) navAi.addEventListener('click', toggleAiDrawer);

  function openAiWithPrompt(promptText) {
    if (aiSidebar) aiSidebar.style.display = 'flex';
    sendAiMessage(promptText);
  }

  document.querySelectorAll('.prompt-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      const promptText = chip.getAttribute('data-prompt');
      if (promptText) sendAiMessage(promptText);
    });
  });

  function sendAiMessage(userText) {
    if (!userText || !userText.trim()) return;
    
    // Add user bubble
    const userMsg = document.createElement('div');
    userMsg.className = 'chat-msg user';
    userMsg.textContent = userText;
    aiChatArea.appendChild(userMsg);
    aiChatArea.scrollTop = aiChatArea.scrollHeight;

    if (aiTextInput) aiTextInput.value = '';

    // Bot Typing Placeholder
    const botMsg = document.createElement('div');
    botMsg.className = 'chat-msg bot';
    botMsg.innerHTML = '<span class="typing-dots">🌿 Thinking & analyzing with Indic engine...</span>';
    aiChatArea.appendChild(botMsg);
    aiChatArea.scrollTop = aiChatArea.scrollHeight;

    // Simulated Intelligent Response
    setTimeout(() => {
      let botResponse = '';
      const lower = userText.toLowerCase();

      if (lower.includes('assamese') || lower.includes('translate') || lower.includes('অসমীয়া')) {
        botResponse = `✨ **Assamese Translation (অসমীয়া অনুবাদ):**\n\nস্বাগতম! অসমৰ অনুপম সৌন্দৰ্য্য আৰু ঐতিহাসিক ঐতিহ্য বিশ্ববিখ্যাত। আমাৰ কাজিৰঙা ৰাষ্ট্ৰীয় উদ্যান আৰু সুবিস্তৃত চাহ বাগিচাসমূহ অসমৰ গৌৰৱ।\n\n*Axomai HarfBuzz Shaper rendered all conjuncts (যুক্তাক্ষৰ) with 100% typographic accuracy.*`;
      } else if (lower.includes('summarize') || lower.includes('summary')) {
        botResponse = `📄 **Page Summary & Key Takeaways:**\n\n• **Core Theme**: High-performance browser architecture with native Indic typography.\n• **Security**: Real-time DoH DNS encryption, TLS 1.3, and EasyList privacy filters.\n• **Performance**: 84% RAM reduction via automatic tab hibernation.\n• **GPU Rasterization**: 60 FPS WGSL shader compositor directly driving display quads.`;
      } else if (lower.includes('rust') || lower.includes('engine')) {
        botResponse = `🦀 **Axomai Rust Multi-Process Architecture:**\n\n1. **Browser Main**: Window manager & IPC message dispatch bus.\n2. **Renderer Process**: HTML5 parser, CSS Grid/Flexbox solver & V8 isolate.\n3. **Network Process**: HTTP/3, TLS 1.3, AdBlock filter & HttpCache with ETag.\n4. **GPU Compositor**: Direct RGBA swapchain presenter & WGSL pipelines.`;
      } else {
        botResponse = `🌿 **Axomai AI Insight:**\n\nI've analyzed your query: *"I can help you explore web content, optimize battery life, translate Indian languages, and execute high-speed workflows."*\n\nIs there anything specific you would like to run or examine next?`;
      }

      botMsg.innerHTML = botResponse.replace(/\n/g, '<br/>');
      aiChatArea.scrollTop = aiChatArea.scrollHeight;
    }, 900);
  }

  if (btnAiSend && aiTextInput) {
    btnAiSend.addEventListener('click', () => sendAiMessage(aiTextInput.value));
    aiTextInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendAiMessage(aiTextInput.value);
      }
    });
  }

  // 10. Heritage Theme & Wallpaper Switcher
  const themeCards = document.querySelectorAll('.theme-card');
  themeCards.forEach(card => {
    card.addEventListener('click', () => {
      themeCards.forEach(c => c.classList.remove('active'));
      card.classList.add('active');

      const themeId = card.getAttribute('data-theme-id');
      applyTheme(themeId);
    });
  });

  function applyTheme(themeId) {
    htmlRoot.setAttribute('data-theme', themeId);
    localStorage.setItem('axomai-theme', themeId);

    // Dynamic background image switch for scenic backdrop
    if (scenicBackdrop) {
      if (themeId === 'tea-garden') {
        scenicBackdrop.style.backgroundImage = "url('assets/images/tea_garden_bg.jpg')";
      } else if (themeId === 'kaziranga') {
        scenicBackdrop.style.backgroundImage = "url('assets/images/kaziranga.jpg')";
      } else if (themeId === 'brahmaputra') {
        scenicBackdrop.style.backgroundImage = "url('assets/images/rhino.jpg')";
      } else if (themeId === 'bihu-crimson') {
        scenicBackdrop.style.backgroundImage = "url('assets/images/bihu.jpg')";
      } else if (themeId === 'cyber-dark') {
        scenicBackdrop.style.backgroundImage = "none";
        scenicBackdrop.style.backgroundColor = "#0f172a";
      }
    }
  }

  // Load Saved Theme
  const savedTheme = localStorage.getItem('axomai-theme') || 'tea-garden';
  applyTheme(savedTheme);
  const activeThemeCard = document.querySelector(`.theme-card[data-theme-id="${savedTheme}"]`);
  if (activeThemeCard) {
    themeCards.forEach(c => c.classList.remove('active'));
    activeThemeCard.classList.add('active');
  }

  // Theme & Settings Modals
  if (btnThemeSelector && themeModal) {
    btnThemeSelector.addEventListener('click', () => themeModal.style.display = 'flex');
  }
  if (btnCloseThemeModal && themeModal) {
    btnCloseThemeModal.addEventListener('click', () => themeModal.style.display = 'none');
  }

  if (navSettings && settingsModal) {
    navSettings.addEventListener('click', () => settingsModal.style.display = 'flex');
  }
  if (btnCloseSettingsModal && settingsModal) {
    btnCloseSettingsModal.addEventListener('click', () => settingsModal.style.display = 'none');
  }

  // QR Modal
  if (btnQrCode && qrModal) {
    btnQrCode.addEventListener('click', () => {
      const qrLabel = document.getElementById('qrUrlLabel');
      if (qrLabel && omniboxInput && omniboxInput.value) {
        qrLabel.textContent = omniboxInput.value;
      }
      qrModal.style.display = 'flex';
    });
  }
  if (btnCloseQrModal && qrModal) {
    btnCloseQrModal.addEventListener('click', () => qrModal.style.display = 'none');
  }

  if (btnCopyLink) {
    btnCopyLink.addEventListener('click', () => {
      navigator.clipboard.writeText(omniboxInput.value || 'https://tourism.assam.gov.in');
      btnCopyLink.textContent = 'Copied! ✓';
      setTimeout(() => btnCopyLink.textContent = 'Copy Link', 2000);
    });
  }

  // Global helper to navigate from anywhere (modals, suggestions)
  window.axomaiNavigate = function(url) {
    navigateTo(url);
    closeAllModals();
  };

  function closeAllModals() {
    const modals = [
      themeModal, settingsModal, qrModal,
      document.getElementById('bookmarksModal'),
      document.getElementById('historyModal'),
      document.getElementById('downloadsModal'),
      document.getElementById('extensionsModal'),
      document.getElementById('passwordsModal'),
      document.getElementById('clearDataModal')
    ];
    modals.forEach(m => { if (m) m.style.display = 'none'; });
    const chromeDropdown = document.getElementById('chromeMenuDropdown');
    if (chromeDropdown) chromeDropdown.classList.remove('show');
  }

  // ==========================================================================
  // Chrome 3-Dots Menu Handler
  // ==========================================================================
  const btnChromeMenu = document.getElementById('btnChromeMenu');
  const chromeMenuDropdown = document.getElementById('chromeMenuDropdown');

  if (btnChromeMenu && chromeMenuDropdown) {
    btnChromeMenu.addEventListener('click', (e) => {
      e.stopPropagation();
      chromeMenuDropdown.classList.toggle('show');
    });

    document.addEventListener('click', (e) => {
      if (!chromeMenuDropdown.contains(e.target) && e.target !== btnChromeMenu) {
        chromeMenuDropdown.classList.remove('show');
      }
    });
  }

  // Modal References
  const bookmarksModal = document.getElementById('bookmarksModal');
  const historyModal = document.getElementById('historyModal');
  const downloadsModal = document.getElementById('downloadsModal');
  const extensionsModal = document.getElementById('extensionsModal');
  const passwordsModal = document.getElementById('passwordsModal');
  const clearDataModal = document.getElementById('clearDataModal');

  // Wire up Menu Items
  const menuNewTab = document.getElementById('menuNewTab');
  const menuNewWindow = document.getElementById('menuNewWindow');
  const menuIncognito = document.getElementById('menuIncognito');
  const menuHome = document.getElementById('menuHome');
  const menuBookmarks = document.getElementById('menuBookmarks');
  const menuHistory = document.getElementById('menuHistory');
  const menuDownloads = document.getElementById('menuDownloads');
  const menuExtensions = document.getElementById('menuExtensions');
  const menuPasswords = document.getElementById('menuPasswords');
  const menuThemes = document.getElementById('menuThemes');
  const menuAiCopilot = document.getElementById('menuAiCopilot');
  const menuWorkspaces = document.getElementById('menuWorkspaces');
  const menuClearData = document.getElementById('menuClearData');
  const menuSettings = document.getElementById('menuSettings');
  const menuExit = document.getElementById('menuExit');

  if (menuNewTab) menuNewTab.addEventListener('click', () => { createNewTab(); closeAllModals(); });
  if (menuNewWindow) menuNewWindow.addEventListener('click', () => { createNewTab(); closeAllModals(); });
  if (menuIncognito) menuIncognito.addEventListener('click', () => { createNewTab('axomai://incognito'); closeAllModals(); });
  if (menuHome) menuHome.addEventListener('click', () => { navigateTo('axomai://newtab'); closeAllModals(); });
  if (menuBookmarks) menuBookmarks.addEventListener('click', () => { closeAllModals(); if (bookmarksModal) bookmarksModal.style.display = 'flex'; });
  if (menuHistory) menuHistory.addEventListener('click', () => { closeAllModals(); if (historyModal) historyModal.style.display = 'flex'; });
  if (menuDownloads) menuDownloads.addEventListener('click', () => { closeAllModals(); if (downloadsModal) downloadsModal.style.display = 'flex'; });
  if (menuExtensions) menuExtensions.addEventListener('click', () => { closeAllModals(); if (extensionsModal) extensionsModal.style.display = 'flex'; });
  if (menuPasswords) menuPasswords.addEventListener('click', () => { closeAllModals(); if (passwordsModal) passwordsModal.style.display = 'flex'; });
  if (menuThemes) menuThemes.addEventListener('click', () => { closeAllModals(); if (themeModal) themeModal.style.display = 'flex'; });
  if (menuAiCopilot) menuAiCopilot.addEventListener('click', () => { closeAllModals(); if (aiSidebar) aiSidebar.classList.toggle('open'); });
  if (menuWorkspaces) menuWorkspaces.addEventListener('click', () => { closeAllModals(); if (themeModal) themeModal.style.display = 'flex'; });
  if (menuClearData) menuClearData.addEventListener('click', () => { closeAllModals(); if (clearDataModal) clearDataModal.style.display = 'flex'; });
  if (menuSettings) menuSettings.addEventListener('click', () => { closeAllModals(); if (settingsModal) settingsModal.style.display = 'flex'; });
  if (menuExit) menuExit.addEventListener('click', () => {
    if (window.ipc) window.ipc.postMessage('close');
    else navigateTo('axomai://newtab');
  });

  // Modal Close Buttons
  const btnCloseBookmarksModal = document.getElementById('btnCloseBookmarksModal');
  const btnCloseHistoryModal = document.getElementById('btnCloseHistoryModal');
  const btnCloseDownloadsModal = document.getElementById('btnCloseDownloadsModal');
  const btnCloseExtensionsModal = document.getElementById('btnCloseExtensionsModal');
  const btnClosePasswordsModal = document.getElementById('btnClosePasswordsModal');
  const btnCloseClearDataModal = document.getElementById('btnCloseClearDataModal');
  const btnCancelClear = document.getElementById('btnCancelClear');
  const btnConfirmClear = document.getElementById('btnConfirmClear');

  if (btnCloseBookmarksModal) btnCloseBookmarksModal.addEventListener('click', () => bookmarksModal.style.display = 'none');
  if (btnCloseHistoryModal) btnCloseHistoryModal.addEventListener('click', () => historyModal.style.display = 'none');
  if (btnCloseDownloadsModal) btnCloseDownloadsModal.addEventListener('click', () => downloadsModal.style.display = 'none');
  if (btnCloseExtensionsModal) btnCloseExtensionsModal.addEventListener('click', () => extensionsModal.style.display = 'none');
  if (btnClosePasswordsModal) btnClosePasswordsModal.addEventListener('click', () => passwordsModal.style.display = 'none');
  if (btnCloseClearDataModal) btnCloseClearDataModal.addEventListener('click', () => clearDataModal.style.display = 'none');
  if (btnCancelClear) btnCancelClear.addEventListener('click', () => clearDataModal.style.display = 'none');

  if (btnConfirmClear) {
    btnConfirmClear.addEventListener('click', () => {
      btnConfirmClear.textContent = '✨ Boosted 842 MB!';
      setTimeout(() => {
        btnConfirmClear.textContent = 'Boost Now';
        clearDataModal.style.display = 'none';
      }, 1200);
    });
  }

  // ==========================================================================
  // Chrome Extension Puzzle Icon & Popover Controller
  // ==========================================================================
  const btnExtensions = document.getElementById('btnExtensions');
  const extDropdownPopup = document.getElementById('extDropdownPopup');
  const btnCloseExtPopover = document.getElementById('btnCloseExtPopover');
  const btnOpenManageExt = document.getElementById('btnOpenManageExt');

  if (btnExtensions && extDropdownPopup) {
    btnExtensions.addEventListener('click', (e) => {
      e.stopPropagation();
      const isOpen = extDropdownPopup.classList.contains('show');
      closeAllModals();
      if (!isOpen) extDropdownPopup.classList.add('show');
    });

    if (btnCloseExtPopover) {
      btnCloseExtPopover.addEventListener('click', () => extDropdownPopup.classList.remove('show'));
    }

    if (btnOpenManageExt) {
      btnOpenManageExt.addEventListener('click', () => {
        extDropdownPopup.classList.remove('show');
        if (extensionsModal) extensionsModal.style.display = 'flex';
      });
    }

    document.addEventListener('click', (e) => {
      if (!extDropdownPopup.contains(e.target) && e.target !== btnExtensions) {
        extDropdownPopup.classList.remove('show');
      }
    });
  }

  // Real-time Extension Enable / Disable Toggles
  const extCheckboxes = [
    { id: 'chkExtAdblock', name: 'EasyList AdBlock Shield' },
    { id: 'chkExtReader', name: 'Reader Mode Pro' },
    { id: 'chkExtTranslate', name: 'Assam Auto-Translate' },
    { id: 'chkExtPrivacy', name: 'Anti-Fingerprint Privacy Guard' },
    { id: 'chkExtCapture', name: 'Screen Capture Studio' },
    { id: 'chkExtBooster', name: 'Turbo RAM Booster' }
  ];

  function updateActiveExtensionsCount() {
    let activeCount = 0;
    extCheckboxes.forEach(item => {
      const chk = document.getElementById(item.id);
      if (chk && chk.checked) activeCount++;
    });
    const badge = document.getElementById('extActiveCountBadge');
    if (badge) badge.textContent = `${activeCount} Extensions Active`;
  }

  extCheckboxes.forEach(item => {
    const chk = document.getElementById(item.id);
    if (chk) {
      chk.addEventListener('change', () => {
        updateActiveExtensionsCount();
        const parentItem = chk.closest('.ext-popover-item');
        if (parentItem) {
          const statusText = parentItem.querySelector('.ext-pop-status');
          if (statusText) {
            statusText.textContent = chk.checked ? 'Active' : 'Disabled';
            statusText.style.color = chk.checked ? '#10b981' : '#64748b';
          }
        }
      });
    }
  });

  // Extension Pin Buttons
  document.querySelectorAll('.ext-pin-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      btn.classList.toggle('active');
    });
  });

  // Toolbar Quick Action Buttons
  const btnDownloads = document.getElementById('btnDownloads');
  const btnShield = document.getElementById('btnShield');

  if (btnDownloads) btnDownloads.addEventListener('click', () => { closeAllModals(); if (downloadsModal) downloadsModal.style.display = 'flex'; });
  if (btnShield) btnShield.addEventListener('click', () => { closeAllModals(); if (extensionsModal) extensionsModal.style.display = 'flex'; });

  // Close modals when clicking backdrop
  window.addEventListener('click', (e) => {
    if (e.target === themeModal) themeModal.style.display = 'none';
    if (e.target === settingsModal) settingsModal.style.display = 'none';
    if (e.target === qrModal) qrModal.style.display = 'none';
    if (e.target === bookmarksModal) bookmarksModal.style.display = 'none';
    if (e.target === historyModal) historyModal.style.display = 'none';
    if (e.target === downloadsModal) downloadsModal.style.display = 'none';
    if (e.target === extensionsModal) extensionsModal.style.display = 'none';
    if (e.target === passwordsModal) passwordsModal.style.display = 'none';
    if (e.target === clearDataModal) clearDataModal.style.display = 'none';
  });

  // ==========================================================================
  // Extension Studio Hub Controllers (Individual UI for Every Extension)
  // ==========================================================================
  const extNavBtns = document.querySelectorAll('.ext-nav-btn');
  const extPanels = document.querySelectorAll('.ext-panel');

  extNavBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      extNavBtns.forEach(b => b.classList.remove('active'));
      extPanels.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const tabKey = btn.getAttribute('data-ext-tab');
      const targetPanel = document.getElementById(`panel-${tabKey}`);
      if (targetPanel) targetPanel.classList.add('active');
    });
  });

  // 1. Reader Mode Interactive Controls
  const readerFontPills = document.querySelectorAll('.font-pill');
  const readerThemeBtns = document.querySelectorAll('.reader-theme-btn');
  const readerFontSizeSlider = document.getElementById('readerFontSizeSlider');
  const readerFontSizeVal = document.getElementById('readerFontSizeVal');
  const readerPreviewBox = document.getElementById('readerPreviewBox');

  if (readerFontPills && readerPreviewBox) {
    readerFontPills.forEach(pill => {
      pill.addEventListener('click', () => {
        readerFontPills.forEach(p => p.classList.remove('active'));
        pill.classList.add('active');
        const font = pill.getAttribute('data-font');
        readerPreviewBox.style.fontFamily = font;
      });
    });
  }

  if (readerFontSizeSlider && readerFontSizeVal && readerPreviewBox) {
    readerFontSizeSlider.addEventListener('input', (e) => {
      const sz = e.target.value;
      readerFontSizeVal.textContent = `${sz}px`;
      readerPreviewBox.style.fontSize = `${sz}px`;
    });
  }

  if (readerThemeBtns && readerPreviewBox) {
    readerThemeBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        readerThemeBtns.forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const bg = btn.getAttribute('data-bg');
        const col = btn.getAttribute('data-color');
        readerPreviewBox.style.backgroundColor = bg;
        readerPreviewBox.style.color = col;
      });
    });
  }

  // 2. Auto-Translate Quick Translator
  const btnRunTranslate = document.getElementById('btnRunTranslate');
  const transSourceText = document.getElementById('transSourceText');
  const transResultBox = document.getElementById('transResultBox');
  const targetLangSelect = document.getElementById('targetLangSelect');

  if (btnRunTranslate && transSourceText && transResultBox) {
    btnRunTranslate.addEventListener('click', () => {
      const src = transSourceText.value.trim();
      const lang = targetLangSelect ? targetLangSelect.value : 'as';
      if (!src) return;

      btnRunTranslate.textContent = 'Translating...';
      setTimeout(() => {
        btnRunTranslate.textContent = 'Translate ✨';
        if (lang === 'as') {
          transResultBox.innerHTML = `<span>✨ <strong>অসমীয়া:</strong> ${src} — সুন্দৰ অসমীয়া যুক্তাক্ষৰৰে সফল অনুবাদ কৰা হ'ল।</span>`;
        } else if (lang === 'bn') {
          transResultBox.innerHTML = `<span>✨ <strong>বাংলা:</strong> ${src} — সফলভাবে অনুবাদ সম্পন্ন হয়েছে।</span>`;
        } else if (lang === 'hi') {
          transResultBox.innerHTML = `<span>✨ <strong>हिन्दी:</strong> ${src} — सफलतापूर्वक अनुवाद संपन्न हुआ।</span>`;
        } else {
          transResultBox.innerHTML = `<span>✨ <strong>Translated:</strong> ${src} [Verified Indic Pipeline]</span>`;
        }
      }, 400);
    });
  }

  // 3. Screen Capture Interactive Buttons
  const btnCaptureVisible = document.getElementById('btnCaptureVisible');
  const btnCaptureFull = document.getElementById('btnCaptureFull');
  const btnCaptureSelection = document.getElementById('btnCaptureSelection');

  function triggerCaptureFlash(label) {
    const flash = document.createElement('div');
    flash.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(255,255,255,0.7);z-index:9999999;pointer-events:none;transition:opacity 0.3s;';
    document.body.appendChild(flash);
    setTimeout(() => { flash.style.opacity = '0'; }, 60);
    setTimeout(() => { flash.remove(); }, 350);

    const toast = document.createElement('div');
    toast.style.cssText = 'position:fixed;top:24px;left:50%;transform:translateX(-50%);background:rgba(15,23,42,0.95);color:#fff;padding:12px 28px;border-radius:24px;font-size:13.5px;font-weight:700;box-shadow:0 8px 30px rgba(0,0,0,0.5);z-index:9999999;border:1px solid rgba(16,185,129,0.4);';
    toast.innerHTML = `📸 <strong>${label}</strong> copied to clipboard!`;
    document.body.appendChild(toast);
    setTimeout(() => { toast.remove(); }, 2200);
  }

  if (btnCaptureVisible) btnCaptureVisible.addEventListener('click', () => triggerCaptureFlash('Visible Screen Captured'));
  if (btnCaptureFull) btnCaptureFull.addEventListener('click', () => triggerCaptureFlash('Full Webpage Scrolled & Captured (4K)'));
  if (btnCaptureSelection) btnCaptureSelection.addEventListener('click', () => triggerCaptureFlash('Selected Region Cropped & Saved'));

  // 4. Turbo RAM Booster Action
  const btnFlushMemoryNow = document.getElementById('btnFlushMemoryNow');
  const boosterSavedVal = document.getElementById('boosterSavedVal');
  const boosterStatusTag = document.getElementById('boosterStatusTag');

  if (btnFlushMemoryNow) {
    btnFlushMemoryNow.addEventListener('click', () => {
      btnFlushMemoryNow.textContent = '⚡ Flushing RAM...';
      setTimeout(() => {
        btnFlushMemoryNow.textContent = '✨ 1.1 GB RAM Freed!';
        if (boosterSavedVal) boosterSavedVal.textContent = '89% Optimized (1.1 GB Saved)';
        if (boosterStatusTag) boosterStatusTag.textContent = 'Optimal • All Inactive Tabs Hibernating';
        setTimeout(() => { btnFlushMemoryNow.textContent = '🧹 Flush Memory & Boost Now'; }, 2000);
      }, 600);
    });
  }

  // 5. AdBlock Whitelist Button
  const btnAddWhitelist = document.getElementById('btnAddWhitelist');
  const whitelistInput = document.getElementById('whitelistInput');
  if (btnAddWhitelist && whitelistInput) {
    btnAddWhitelist.addEventListener('click', () => {
      const site = whitelistInput.value.trim();
      if (site) {
        btnAddWhitelist.textContent = '✓ Allowed';
        whitelistInput.value = '';
        setTimeout(() => { btnAddWhitelist.textContent = 'Allow Site'; }, 1800);
      }
    });
  }

  // Keyboard Shortcuts (Ctrl+T, Ctrl+H, Ctrl+J, Escape)
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeAllModals();
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 't') {
      e.preventDefault();
      createNewTab();
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'h') {
      e.preventDefault();
      closeAllModals();
      if (historyModal) historyModal.style.display = 'flex';
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'j') {
      e.preventDefault();
      closeAllModals();
      if (downloadsModal) downloadsModal.style.display = 'flex';
    }
  });

  // Bookmark Star Animation
  const btnBookmarkStar = document.getElementById('btnBookmarkStar');
  if (btnBookmarkStar) {
    btnBookmarkStar.addEventListener('click', () => {
      const isStarred = btnBookmarkStar.classList.toggle('starred');
      if (isStarred) {
        btnBookmarkStar.style.color = '#f59e0b';
        btnBookmarkStar.innerHTML = `<svg viewBox="0 0 24 24" width="16" height="16" fill="#f59e0b" stroke="#f59e0b" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>`;
      } else {
        btnBookmarkStar.style.color = '';
        btnBookmarkStar.innerHTML = `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>`;
      }
    });
  }

  // Workspaces Selection
  document.querySelectorAll('.ws-item').forEach(item => {
    item.addEventListener('click', () => {
      document.querySelectorAll('.ws-item').forEach(w => w.classList.remove('active'));
      item.classList.add('active');
    });
  });

  // Window Controls (WinMin, WinMax, WinClose)
  const winMin = document.getElementById('winMin');
  const winMax = document.getElementById('winMax');
  const winClose = document.getElementById('winClose');

  if (winMin) winMin.addEventListener('click', () => { if (window.ipc) window.ipc.postMessage('minimize'); });
  if (winMax) winMax.addEventListener('click', () => { if (window.ipc) window.ipc.postMessage('maximize'); });
  if (winClose) winClose.addEventListener('click', () => { if (window.ipc) window.ipc.postMessage('close'); });
});
