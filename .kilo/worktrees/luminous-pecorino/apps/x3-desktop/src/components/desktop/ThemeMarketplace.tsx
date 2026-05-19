// Theme marketplace: browse, download, and install custom themes
// Users can create, share, and rate themes

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface ThemeColors {
  primary: string;
  secondary: string;
  success: string;
  danger: string;
  warning: string;
  info: string;
  light: string;
  dark: string;
  background: string;
  surface: string;
  border: string;
  text: string;
  text_secondary: string;
  accent: string;
}

export interface ThemeFonts {
  primary: string;
  mono: string;
  size_base: number;
  size_sm: number;
  size_lg: number;
  size_xl: number;
  weight_normal: number;
  weight_bold: number;
}

export interface Theme {
  id: string;
  name: string;
  description: string;
  variant: 'light' | 'dark' | 'high-contrast' | 'custom';
  colors: ThemeColors;
  fonts: ThemeFonts;
  author: string;
  version: string;
  rating: number;
  downloads: number;
  created_at: Date;
  updated_at: Date;
  is_builtin: boolean;
  is_installed: boolean;
  preview_url?: string;
}

interface MarketplaceState {
  marketplace_themes: Theme[];
  installed_themes: Theme[];
  active_theme: string;
  loading: boolean;
  search_query: string;
  selected_variant: 'all' | 'light' | 'dark' | 'high-contrast' | 'custom';
  sort_by: 'rating' | 'downloads' | 'newest' | 'name';
}

const DEFAULT_THEMES: Theme[] = [
  {
    id: 'buil-light',
    name: 'Light',
    description: 'Clean light theme optimized for daytime use',
    variant: 'light',
    colors: {
      primary: '#0066cc',
      secondary: '#6c757d',
      success: '#28a745',
      danger: '#dc3545',
      warning: '#ffc107',
      info: '#17a2b8',
      light: '#f8f9fa',
      dark: '#343a40',
      background: '#ffffff',
      surface: '#f5f5f5',
      border: '#ddd',
      text: '#333333',
      text_secondary: '#666666',
      accent: '#ff6600',
    },
    fonts: {
      primary: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto',
      mono: '"SF Mono", Monaco, "Cascadia Code", Roboto Mono',
      size_base: 14,
      size_sm: 12,
      size_lg: 16,
      size_xl: 20,
      weight_normal: 400,
      weight_bold: 600,
    },
    author: 'X3 Team',
    version: '1.0.0',
    rating: 4.8,
    downloads: 15000,
    created_at: new Date(),
    updated_at: new Date(),
    is_builtin: true,
    is_installed: true,
  },
  {
    id: 'builtin-dark',
    name: 'Dark',
    description: 'Dark theme with OLED-friendly colors for night use',
    variant: 'dark',
    colors: {
      primary: '#0066cc',
      secondary: '#6c757d',
      success: '#28a745',
      danger: '#dc3545',
      warning: '#ffc107',
      info: '#17a2b8',
      light: '#f8f9fa',
      dark: '#1a1a1a',
      background: '#0c0c0c',
      surface: '#1e1e1e',
      border: '#333',
      text: '#e0e0e0',
      text_secondary: '#999',
      accent: '#ff6600',
    },
    fonts: {
      primary: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto',
      mono: '"SF Mono", Monaco, "Cascadia Code", Roboto Mono',
      size_base: 14,
      size_sm: 12,
      size_lg: 16,
      size_xl: 20,
      weight_normal: 400,
      weight_bold: 600,
    },
    author: 'X3 Team',
    version: '1.0.0',
    rating: 4.9,
    downloads: 20000,
    created_at: new Date(),
    updated_at: new Date(),
    is_builtin: true,
    is_installed: true,
  },
];

export const ThemeMarketplace: React.FC = () => {
  const [state, setState] = useState<MarketplaceState>({
    marketplace_themes: DEFAULT_THEMES,
    installed_themes: DEFAULT_THEMES.filter(t => t.is_builtin),
    active_theme: 'builtin-dark',
    loading: false,
    search_query: '',
    selected_variant: 'all',
    sort_by: 'rating',
  });

  const [showMarketplace, setShowMarketplace] = useState(false);
  const [selectedTheme, setSelectedTheme] = useState<Theme | null>(null);
  const [showPreview, setShowPreview] = useState(false);

  // Load themes from marketplace
  useEffect(() => {
    const loadMarketplaceThemes = async () => {
      setState(prev => ({ ...prev, loading: true }));
      try {
        const themes = await invoke<Theme[]>('fetch_theme_marketplace');
        setState(prev => ({
          ...prev,
          marketplace_themes: [...DEFAULT_THEMES, ...themes],
          loading: false,
        }));
      } catch (error) {
        console.error('Failed to load marketplace themes:', error);
        setState(prev => ({ ...prev, loading: false }));
      }
    };

    if (showMarketplace) {
      loadMarketplaceThemes();
    }
  }, [showMarketplace]);

  // Load installed themes
  useEffect(() => {
    const loadInstalledThemes = async () => {
      try {
        const themes = await invoke<Theme[]>('get_installed_themes');
        setState(prev => ({
          ...prev,
          installed_themes: themes,
        }));
      } catch (error) {
        console.error('Failed to load installed themes:', error);
      }
    };

    loadInstalledThemes();
  }, []);

  // Install theme
  const installTheme = async (theme: Theme) => {
    try {
      await invoke('install_theme', { theme_id: theme.id });
      setState(prev => ({
        ...prev,
        installed_themes: [...prev.installed_themes, theme],
      }));
    } catch (error) {
      console.error('Failed to install theme:', error);
    }
  };

  // Uninstall theme
  const uninstallTheme = async (themeId: string) => {
    try {
      await invoke('uninstall_theme', { theme_id: themeId });
      setState(prev => ({
        ...prev,
        installed_themes: prev.installed_themes.filter(t => t.id !== themeId),
      }));
    } catch (error) {
      console.error('Failed to uninstall theme:', error);
    }
  };

  // Apply theme
  const applyTheme = async (themeId: string) => {
    try {
      const theme = state.marketplace_themes.find(t => t.id === themeId);
      if (!theme) return;

      await invoke('apply_theme', { theme_id: themeId, theme });
      setState(prev => ({ ...prev, active_theme: themeId }));
      applyThemeToDOM(theme);
    } catch (error) {
      console.error('Failed to apply theme:', error);
    }
  };

  // Apply theme colors to DOM
  const applyThemeToDOM = (theme: Theme) => {
    const root = document.documentElement;
    const colors = theme.colors;
    const fonts = theme.fonts;

    // Set CSS variables for theme colors
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--color-${key}`, value);
    });

    // Set CSS variables for fonts
    Object.entries(fonts).forEach(([key, value]) => {
      if (typeof value === 'string') {
        root.style.setProperty(`--font-${key}`, value);
      } else {
        root.style.setProperty(`--font-${key}`, `${value}px`);
      }
    });
  };

  // Rate theme
  const rateTheme = async (themeId: string, rating: number) => {
    try {
      await invoke('rate_theme', { theme_id: themeId, rating });
    } catch (error) {
      console.error('Failed to rate theme:', error);
    }
  };

  // Filter and sort themes
  const filteredThemes = state.marketplace_themes
    .filter(theme => {
      const matchesSearch = theme.name.toLowerCase().includes(state.search_query.toLowerCase()) ||
        theme.description.toLowerCase().includes(state.search_query.toLowerCase());
      const matchesVariant = state.selected_variant === 'all' || theme.variant === state.selected_variant;
      return matchesSearch && matchesVariant;
    })
    .sort((a, b) => {
      switch (state.sort_by) {
        case 'rating':
          return b.rating - a.rating;
        case 'downloads':
          return b.downloads - a.downloads;
        case 'newest':
          return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        case 'name':
          return a.name.localeCompare(b.name);
        default:
          return 0;
      }
    });

  return (
    <div className="theme-marketplace">
      <div className="theme-header">
        <h3>🎨 Themes</h3>
        <button
          className="btn-marketplace"
          onClick={() => setShowMarketplace(!showMarketplace)}
        >
          {showMarketplace ? 'Hide' : 'Browse'} Marketplace
        </button>
      </div>

      {/* Installed Themes */}
      <div className="installed-themes">
        <h4>Installed Themes ({state.installed_themes.length})</h4>
        <div className="theme-grid">
          {state.installed_themes.map(theme => (
            <div
              key={theme.id}
              className={`theme-card ${theme.id === state.active_theme ? 'active' : ''}`}
            >
              <div className="theme-preview" style={{
                background: theme.colors.background,
                color: theme.colors.text,
              }}>
                <div style={{ fontSize: theme.fonts.size_base }}>
                  Sample Text
                </div>
                <div style={{ fontSize: theme.fonts.size_sm, color: theme.colors.text_secondary }}>
                  Secondary Text
                </div>
                <div style={{ marginTop: '8px', display: 'flex', gap: '4px' }}>
                  <span style={{ background: theme.colors.primary, padding: '4px 8px', borderRadius: '3px', color: 'white', fontSize: '11px' }}>Primary</span>
                  <span style={{ background: theme.colors.success, padding: '4px 8px', borderRadius: '3px', color: 'white', fontSize: '11px' }}>Success</span>
                </div>
              </div>
              <div className="theme-info">
                <div className="theme-name">{theme.name}</div>
                <div className="theme-variant">{theme.variant}</div>
                <div className="theme-rating">{'⭐'.repeat(Math.round(theme.rating))}</div>
              </div>
              <div className="theme-actions">
                <button
                  className={`btn-apply ${theme.id === state.active_theme ? 'active' : ''}`}
                  onClick={() => applyTheme(theme.id)}
                  disabled={theme.id === state.active_theme}
                >
                  {theme.id === state.active_theme ? '✓ Active' : 'Apply'}
                </button>
                {!theme.is_builtin && (
                  <button
                    className="btn-remove"
                    onClick={() => uninstallTheme(theme.id)}
                  >
                    Remove
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Marketplace */}
      {showMarketplace && (
        <div className="marketplace-section">
          <h4>Theme Marketplace</h4>

          {/* Filters */}
          <div className="marketplace-filters">
            <input
              type="text"
              placeholder="Search themes..."
              value={state.search_query}
              onChange={e => setState(prev => ({ ...prev, search_query: e.target.value }))}
              className="search-input"
            />

            <select
              value={state.selected_variant}
              onChange={e => setState(prev => ({ ...prev, selected_variant: e.target.value as any }))}
            >
              <option value="all">All Variants</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="high-contrast">High Contrast</option>
              <option value="custom">Custom</option>
            </select>

            <select
              value={state.sort_by}
              onChange={e => setState(prev => ({ ...prev, sort_by: e.target.value as any }))}
            >
              <option value="rating">Top Rated</option>
              <option value="downloads">Most Downloaded</option>
              <option value="newest">Newest</option>
              <option value="name">A-Z</option>
            </select>
          </div>

          {/* Loading State */}
          {state.loading && (
            <div className="loading-state">Loading themes...</div>
          )}

          {/* Themes Grid */}
          {!state.loading && (
            <div className="theme-grid-marketplace">
              {filteredThemes.map(theme => (
                <div
                  key={theme.id}
                  className="theme-card-marketplace"
                  onMouseEnter={() => setSelectedTheme(theme)}
                  onMouseLeave={() => setSelectedTheme(null)}
                >
                  <div className="theme-card-header">
                    <div className="theme-name">{theme.name}</div>
                    <div className="theme-rating">⭐ {theme.rating.toFixed(1)}</div>
                  </div>

                  <div className="theme-preview-preview" style={{
                    background: theme.colors.background,
                    color: theme.colors.text,
                  }}>
                    <div>Preview</div>
                  </div>

                  <div className="theme-card-meta">
                    <div className="theme-author">By {theme.author}</div>
                    <div className="theme-downloads">📥 {theme.downloads.toLocaleString()}</div>
                  </div>

                  <div className="theme-card-description">
                    {theme.description}
                  </div>

                  <div className="theme-card-actions">
                    {theme.is_installed ? (
                      <button
                        className={`btn-apply-marketplace ${theme.id === state.active_theme ? 'active' : ''}`}
                        onClick={() => applyTheme(theme.id)}
                        disabled={theme.id === state.active_theme}
                      >
                        {theme.id === state.active_theme ? '✓ Active' : 'Apply'}
                      </button>
                    ) : (
                      <button
                        className="btn-install"
                        onClick={() => installTheme(theme)}
                      >
                        📥 Install
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <style>{`
        .theme-marketplace {
          background: white;
          border-radius: 8px;
          padding: 20px;
        }

        .theme-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }

        .btn-marketplace {
          padding: 8px 16px;
          background: #0066cc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        }

        .btn-marketplace:hover {
          background: #0052a3;
        }

        .installed-themes h4 {
          margin: 0 0 16px 0;
        }

        .theme-grid,
        .theme-grid-marketplace {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
          gap: 16px;
          margin-bottom: 20px;
        }

        .theme-card {
          background: #f9f9f9;
          border: 2px solid #ddd;
          border-radius: 8px;
          overflow: hidden;
          transition: all 0.2s;
          cursor: pointer;
        }

        .theme-card:hover {
          border-color: #0066cc;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        }

        .theme-card.active {
          border-color: #28a745;
          background: #f0f9ff;
        }

        .theme-preview {
          height: 100px;
          padding: 12px;
          display: flex;
          flex-direction: column;
          justify-content: center;
          gap: 8px;
        }

        .theme-info {
          padding: 12px;
          background: white;
        }

        .theme-name {
          font-weight: 600;
          font-size: 13px;
          margin-bottom: 4px;
        }

        .theme-variant {
          font-size: 11px;
          color: #666;
          margin-bottom: 4px;
        }

        .theme-rating {
          font-size: 12px;
        }

        .theme-actions {
          display: flex;
          gap: 6px;
          padding: 8px;
        }

        .btn-apply,
        .btn-remove,
        .btn-install,
        .btn-apply-marketplace {
          flex: 1;
          padding: 6px;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 11px;
          cursor: pointer;
          background: white;
          transition: all 0.2s;
        }

        .btn-apply:hover,
        .btn-install:hover,
        .btn-apply-marketplace:hover {
          background: #e8e8e8;
        }

        .btn-apply.active,
        .btn-apply-marketplace.active {
          background: #28a745;
          color: white;
          border-color: #28a745;
        }

        .btn-remove {
          background: #ffebee;
          color: #c62828;
          border-color: #ef5350;
        }

        .btn-remove:hover {
          background: #ffcdd2;
        }

        .marketplace-section {
          margin-top: 20px;
          padding-top: 20px;
          border-top: 1px solid #eee;
        }

        .marketplace-filters {
          display: flex;
          gap: 12px;
          margin-bottom: 16px;
        }

        .search-input {
          flex: 1;
          padding: 8px;
          border: 1px solid #ddd;
          border-radius: 4px;
        }

        .marketplace-filters select {
          padding: 8px;
          border: 1px solid #ddd;
          border-radius: 4px;
        }

        .loading-state {
          text-align: center;
          padding: 40px;
          color: #999;
        }

        .theme-grid-marketplace {
          grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        }

        .theme-card-marketplace {
          background: white;
          border: 1px solid #ddd;
          border-radius: 8px;
          overflow: hidden;
          padding: 12px;
          transition: all 0.2s;
        }

        .theme-card-marketplace:hover {
          border-color: #0066cc;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        }

        .theme-card-header {
          display: flex;
          justify-content: space-between;
          align-items: start;
          margin-bottom: 8px;
        }

        .theme-card-meta {
          display: flex;
          justify-content: space-between;
          font-size: 11px;
          color: #666;
          margin: 8px 0;
        }

        .theme-card-description {
          font-size: 12px;
          color: #666;
          margin: 8px 0;
          line-height: 1.4;
        }

        .theme-preview-preview {
          height: 80px;
          border-radius: 4px;
          display: flex;
          align-items: center;
          justify-content: center;
          margin: 8px 0;
          font-size: 12px;
          font-weight: 500;
        }

        .theme-card-actions {
          display: flex;
          gap: 8px;
          margin-top: 8px;
        }
      `}</style>
    </div>
  );
};

export default ThemeMarketplace;
