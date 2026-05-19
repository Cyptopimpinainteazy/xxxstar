/**
 * X3 Navigation Loader
 * Loads and injects the unified navigation system for standalone HTML pages.
 * Works in both web (standalone) and desktop (Tauri iframe) contexts.
 */

(function () {
  'use strict';

  function ensureNavAssets() {
    // Load CSS if not already loaded
    if (!document.querySelector('link[href*="x3-site-nav.css"]')) {
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = 'css/x3-site-nav.css';
      document.head.appendChild(link);
    }

    // Load JS if not already loaded
    if (!window.X3NavLoaded) {
      const script = document.createElement('script');
      script.src = 'js/x3-site-nav.js';
      script.onload = function () {
        window.X3NavLoaded = true;
      };
      document.body.appendChild(script);
    }
  }

  function detectContext() {
    // Check if we're in an iframe (embedded in Tauri)
    const inIframe = window.self !== window.top;
    // Check if we're in a Tauri webview (special Tauri variable exists)
    const inTauri = typeof window.__TAURI__ !== 'undefined';

    return {
      isEmbedded: inIframe || inTauri,
      isStandalone: !inIframe && !inTauri,
    };
  }

  function initStandaloneNav() {
    // For standalone pages, ensure nav is loaded and injected
    ensureNavAssets();
  }

  function initEmbeddedNav() {
    // For embedded pages (Tauri iframe), ensure nav is loaded
    // but also communicate with parent app if needed
    ensureNavAssets();

    // Listen for navigation requests from embedded context
    if (window.self !== window.top) {
      window.addEventListener('message', function (event) {
        // Handle any cross-iframe navigation if needed
        if (event.data.type === 'navigate') {
          window.location.href = event.data.href;
        }
      });
    }
  }

  // Initialize on DOM ready
  function init() {
    const context = detectContext();

    if (context.isStandalone) {
      initStandaloneNav();
    } else if (context.isEmbedded) {
      initEmbeddedNav();
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
