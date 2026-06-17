/* ============================================
   rsplotlib 教程 · 交互行为
   - 主题切换（深色 / 浅色）
   - 侧边栏导航高亮
   - 标签页切换
   - 复制代码
   - 返回顶部
   - 快捷键
   - 移动端侧边栏
   ============================================ */

(function () {
  'use strict';

  /* ---------- 主题切换 ---------- */
  const THEME_KEY = 'rsplotlib-theme';
  const themeToggle = document.getElementById('themeToggle');
  const root = document.documentElement;

  function applyTheme(theme) {
    root.setAttribute('data-theme', theme);
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch (e) {}
  }

  function initTheme() {
    let saved = null;
    try {
      saved = localStorage.getItem(THEME_KEY);
    } catch (e) {}
    if (saved === 'light' || saved === 'dark') {
      applyTheme(saved);
    } else if (window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches) {
      applyTheme('light');
    } else {
      applyTheme('dark');
    }
  }

  if (themeToggle) {
    themeToggle.addEventListener('click', () => {
      const current = root.getAttribute('data-theme') || 'dark';
      applyTheme(current === 'dark' ? 'light' : 'dark');
    });
  }

  initTheme();

  /* ---------- 标签页切换 ---------- */
  document.querySelectorAll('.tabs').forEach((tabGroup) => {
    const buttons = tabGroup.querySelectorAll('.tab-btn');
    const panels = tabGroup.querySelectorAll('.tab-panel');

    buttons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const target = btn.getAttribute('data-tab');

        buttons.forEach((b) => b.classList.toggle('active', b === btn));
        panels.forEach((p) => p.classList.toggle('active', p.getAttribute('data-tab') === target));
      });
    });
  });

  /* ---------- 复制代码 ---------- */
  document.querySelectorAll('[data-copy]').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.preventDefault();
      const card = btn.closest('.code-card');
      const code = card ? card.querySelector('pre code') : null;
      if (!code) return;

      try {
        await navigator.clipboard.writeText(code.textContent);
        const original = btn.textContent;
        btn.textContent = '已复制';
        btn.classList.add('copied');
        setTimeout(() => {
          btn.textContent = original;
          btn.classList.remove('copied');
        }, 1800);
      } catch (err) {
        // 后备方案：选中文本
        const range = document.createRange();
        range.selectNode(code);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        try {
          document.execCommand('copy');
          btn.textContent = '已复制';
          setTimeout(() => { btn.textContent = '复制'; }, 1800);
        } catch (e2) {
          btn.textContent = '复制失败';
        }
        selection.removeAllRanges();
      }
    });
  });

  /* ---------- 侧边栏高亮当前章节 ---------- */
  const tocLinks = document.querySelectorAll('.toc-link');
  const chapters = Array.from(tocLinks)
    .map((link) => {
      const id = link.getAttribute('data-ch');
      const el = document.getElementById(id);
      return el ? { id, el, link } : null;
    })
    .filter(Boolean);

  if (chapters.length > 0 && 'IntersectionObserver' in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const data = chapters.find((c) => c.el === entry.target);
          if (data && entry.isIntersecting) {
            tocLinks.forEach((l) => l.classList.remove('active'));
            data.link.classList.add('active');
          }
        });
      },
      {
        rootMargin: '-20% 0px -70% 0px',
        threshold: 0,
      }
    );

    chapters.forEach((c) => observer.observe(c.el));
  }

  // 点击侧边栏链接在移动端自动关闭
  tocLinks.forEach((link) => {
    link.addEventListener('click', () => {
      if (window.innerWidth <= 768) {
        document.getElementById('sidebar')?.classList.remove('open');
      }
    });
  });

  /* ---------- 移动端侧边栏切换 ---------- */
  const sidebarToggle = document.getElementById('sidebarToggle');
  const sidebar = document.getElementById('sidebar');
  if (sidebarToggle && sidebar) {
    sidebarToggle.addEventListener('click', () => {
      sidebar.classList.toggle('open');
    });

    // 点击外部关闭
    document.addEventListener('click', (e) => {
      if (window.innerWidth <= 768 &&
          sidebar.classList.contains('open') &&
          !sidebar.contains(e.target) &&
          !sidebarToggle.contains(e.target)) {
        sidebar.classList.remove('open');
      }
    });
  }

  /* ---------- 返回顶部 ---------- */
  const backToTop = document.getElementById('backToTop');
  if (backToTop) {
    const toggleBackToTop = () => {
      const scrollY = window.scrollY || document.documentElement.scrollTop;
      backToTop.classList.toggle('show', scrollY > 600);
    };

    window.addEventListener('scroll', toggleBackToTop, { passive: true });
    toggleBackToTop();

    backToTop.addEventListener('click', () => {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    });
  }

  /* ---------- 搜索（基础实现） ---------- */
  const searchInput = document.getElementById('searchInput');
  if (searchInput) {
    let searchIndex = [];

    // 构建可搜索文本索引
    function buildIndex() {
      const items = document.querySelectorAll('article.chapter, .section-title, .api-item');
      searchIndex = Array.from(items).map((el) => {
        const rect = el.getBoundingClientRect();
        return {
          el,
          text: el.textContent.toLowerCase(),
          top: rect.top + window.scrollY,
        };
      });
    }

    buildIndex();

    searchInput.addEventListener('focus', () => {
      if (searchIndex.length === 0) buildIndex();
    });

    let searchTimer = null;
    searchInput.addEventListener('input', (e) => {
      clearTimeout(searchTimer);
      const query = e.target.value.trim().toLowerCase();

      // 重置所有高亮
      document.querySelectorAll('.search-highlight').forEach((h) => {
        const parent = h.parentNode;
        parent.replaceChild(document.createTextNode(h.textContent), h);
        parent.normalize();
      });
      document.querySelectorAll('.toc-link.flash').forEach((l) => l.classList.remove('flash'));

      if (!query) return;

      searchTimer = setTimeout(() => {
        let firstMatch = null;
        searchIndex.forEach((item) => {
          if (item.text.includes(query)) {
            if (!firstMatch) firstMatch = item;
            item.el.classList.add('search-flash');
          }
        });

        if (firstMatch) {
          firstMatch.el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }, 200);
    });
  }

  /* ---------- 快捷键 ---------- */
  document.addEventListener('keydown', (e) => {
    // Cmd/Ctrl + K → 聚焦搜索
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      searchInput?.focus();
    }

    // ESC → 关闭搜索、移动端侧边栏
    if (e.key === 'Escape') {
      if (searchInput && document.activeElement === searchInput) {
        searchInput.value = '';
        searchInput.blur();
      }
      if (window.innerWidth <= 768) {
        document.getElementById('sidebar')?.classList.remove('open');
      }
    }

    // t → 切换主题
    if (e.key === 't' && !e.metaKey && !e.ctrlKey &&
        document.activeElement.tagName !== 'INPUT' &&
        document.activeElement.tagName !== 'TEXTAREA') {
      const current = root.getAttribute('data-theme') || 'dark';
      applyTheme(current === 'dark' ? 'light' : 'dark');
    }
  });

  /* ---------- 进入视口时的淡入动画 ---------- */
  if ('IntersectionObserver' in window) {
    const fadeObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('in-view');
            fadeObserver.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.1, rootMargin: '0px 0px -50px 0px' }
    );

    document.querySelectorAll('.section, .info-card, .code-card, .api-item').forEach((el) => {
      el.classList.add('fade-target');
      fadeObserver.observe(el);
    });
  }

  /* ---------- 控制台彩蛋 ---------- */
  if (window.console && console.log) {
    const style1 = 'color: #ce422b; font-weight: bold; font-size: 18px;';
    const style2 = 'color: #4584b6; font-weight: 500; font-size: 13px;';
    console.log('%crsplotlib %c· Rust + Python 高性能数据可视化', style1, style2);
    console.log('%c版本 v0.1.5 · 2026-06-17', 'color: #6b7785; font-size: 11px;');
  }
})();

// 为淡入动画添加 CSS（动态注入）
(function () {
  const style = document.createElement('style');
  style.textContent = `
    .fade-target {
      opacity: 0;
      transform: translateY(8px);
      transition: opacity 0.5s ease, transform 0.5s ease;
    }
    .fade-target.in-view {
      opacity: 1;
      transform: translateY(0);
    }
    .search-flash {
      animation: flash 1.5s ease;
    }
    @keyframes flash {
      0%, 100% { background-color: transparent; }
      30% { background-color: rgba(206, 66, 43, 0.15); }
    }
    .toc-link.flash {
      color: var(--brand-rust-soft);
      font-weight: 600;
    }
  `;
  document.head.appendChild(style);
})();
