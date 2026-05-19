/**
 * TopNavBar.tsx — Top navigation bar with dropdown menus
 *
 * Features:
 * - File menu with New, Open, Save options
 * - Edit menu with Copy, Paste, Preferences
 * - View menu with themes, layouts
 * - Tools menu with developer options
 * - Help menu with documentation
 */
import React, { useState, useRef, useCallback, useMemo, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTheme } from '../theme/ThemeProvider';
import { useDesktopStore } from '@/stores/desktopStore';
import { useSocialStore } from '@/stores/socialStore';
import { useApplicationStore } from '@/stores/applicationStore';
import { useWindowManager } from '@/hooks/useWindowManager';
import { setAppNetwork } from '@/lib/substrate/client';
import { useWalletStore } from '@/stores/walletStore';
import type { ApplicationCategory } from '@/types/application';

interface MenuItem {
  label?: string;
  icon?: string;
  shortcut?: string;
  action?: () => void;
  divider?: boolean;
  submenu?: MenuItem[];
}

interface DropdownMenuProps {
  items: MenuItem[];
  isOpen: boolean;
  onClose: () => void;
  position: { top: number; left: number };
}

const DropdownMenu: React.FC<DropdownMenuProps> = ({ items, isOpen, onClose, position }) => {
  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
      />

      {/* Menu */}
      <div
        className="absolute glass-panel border border-border-default rounded-lg shadow-2xl z-50 min-w-[200px] py-1"
        style={{ top: position.top, left: position.left }}
      >
        {items.map((item, index) => (
          <React.Fragment key={index}>
            {item.divider && <div className="border-t border-border-default my-1" />}
            {!item.divider && (
              <button
                className="w-full px-4 py-2 text-left text-sm text-text-primary hover:bg-accent-primary/10
                  hover:text-accent-primary transition-colors flex items-center justify-between group"
                onClick={() => {
                  item.action?.();
                  onClose();
                }}
              >
                <div className="flex items-center gap-3">
                  {item.icon && <span className="text-base">{item.icon}</span>}
                  <span>{item.label}</span>
                </div>
                {item.shortcut && (
                  <span className="text-xs text-text-secondary group-hover:text-accent-primary/70">
                    {item.shortcut}
                  </span>
                )}
              </button>
            )}
          </React.Fragment>
        ))}
      </div>
    </>
  );
};

const TopNavBar: React.FC = () => {
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0 });
  const menuRefs = useRef<{ [key: string]: HTMLButtonElement | null }>({});

  const { toggle: toggleTheme, isDark } = useTheme();
  const iconSize = useDesktopStore((s) => s.iconSize);
  const setIconSize = useDesktopStore((s) => s.setIconSize);
  const minimizeAll = useDesktopStore((s) => s.minimizeAll);

  // Social login state
  const { isLoggedIn, currentUser, session, logout: socialLogout, restoreSession } = useSocialStore();
  const navigate = useNavigate();
  const [showLoginDropdown, setShowLoginDropdown] = useState(false);
  const [loginUser, setLoginUser] = useState('');
  const [loginPass, setLoginPass] = useState('');
  const [loginLoading, setLoginLoading] = useState(false);
  const [loginError, setLoginError] = useState('');
  const socialLogin = useSocialStore((s) => s.login);
  const socialLoginWithWallet = useSocialStore((s) => s.loginWithWallet);
  const { universalWallet } = useWalletStore();

  // Restore session on mount
  React.useEffect(() => { restoreSession(); }, []);

  // Application registry & window manager for Apps dropdown
  const applications = useApplicationStore((s) => s.applications);
  const { launch } = useWindowManager();

  const CATEGORY_META: Record<ApplicationCategory, { icon: string; label: string }> = {
    blockchain: { icon: '⛓️', label: 'Blockchain' },
    defi:       { icon: '💰', label: 'DeFi' },
    analysis:   { icon: '📊', label: 'Analysis' },
    service:    { icon: '🔧', label: 'Services' },
    security:   { icon: '🛡️', label: 'Security' },
    development:{ icon: '💻', label: 'Development' },
    utility:    { icon: '🧰', label: 'Utilities' },
    other:      { icon: '📦', label: 'Other' },
  };

  const CATEGORY_ORDER: ApplicationCategory[] = [
    'blockchain', 'defi', 'analysis', 'service', 'security', 'development', 'utility', 'other',
  ];

  const groupedApps = useMemo(() => {
    const groups: Partial<Record<ApplicationCategory, typeof applications>> = {};
    for (const app of applications) {
      const cat = app.category || 'other';
      if (!groups[cat]) groups[cat] = [];
      groups[cat]!.push(app);
    }
    // Sort each group alphabetically by name
    for (const cat of Object.keys(groups) as ApplicationCategory[]) {
      groups[cat]!.sort((a, b) => a.name.localeCompare(b.name));
    }
    return groups;
  }, [applications]);

  const [showAppsMenu, setShowAppsMenu] = useState(false);
  const [appsMenuFilter, setAppsMenuFilter] = useState('');
  const appsButtonRef = useRef<HTMLButtonElement | null>(null);

  // Network selector (persists to localStorage and reconnects Substrate client)
  const NETWORK_OPTIONS = [
    { id: 'local', label: 'Local' },
    { id: 'testnet', label: 'Testnet' },
    { id: 'mainnet', label: 'Mainnet' },
  ] as const;

  const [selectedNetwork, setSelectedNetwork] = useState<string>(() => {
    try {
      if (typeof window !== 'undefined') {
        const stored = window.localStorage.getItem('x3_active_network');
        if (stored) return stored;
      }
    } catch (err) { /* ignore */ }
    const isDev = (typeof process !== 'undefined' && process.env && process.env.NODE_ENV === 'development') || (typeof import.meta !== 'undefined' && (import.meta as any).env?.MODE === 'development');
    return isDev ? 'local' : 'testnet';
  });

  useEffect(() => {
    let mounted = true;
    const apply = async () => {
      try {
        // persist & ask substrate client to reconnect
        await setAppNetwork(selectedNetwork as 'local' | 'testnet' | 'mainnet');
      } catch (err) {
        console.error('[TopNavBar] setAppNetwork failed', err);
      }
    };
    if (mounted) apply();
    return () => { mounted = false; };
  }, [selectedNetwork]);

  const handleQuickLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!loginUser || !loginPass) { setLoginError('Fill in both fields'); return; }
    setLoginLoading(true);
    setLoginError('');
    try {
      await socialLogin(loginUser, loginPass);
      setShowLoginDropdown(false);
      setLoginUser('');
      setLoginPass('');
    } catch (err: any) {
      setLoginError(String(err));
    }
    setLoginLoading(false);
  };

  const handleWalletLogin = async () => {
    if (!universalWallet) return;
    setLoginLoading(true);
    try {
      await socialLoginWithWallet(universalWallet.evm_address);
      setShowLoginDropdown(false);
    } catch (err: any) {
      setLoginError(String(err));
    }
    setLoginLoading(false);
  };

  const handleSignOut = async () => {
    await socialLogout();
    setShowLoginDropdown(false);
  };

  const ROLE_ICON: Record<string, string> = { team: '🔶', admin: '👑', vip: '💎' };
  const ROLE_COLOR: Record<string, string> = { team: '#1a9fb5', admin: '#ff2d55', vip: '#0d5f7a' };

  const handleMenuClick = useCallback((menuName: string, event: React.MouseEvent) => {
    event.preventDefault();
    setShowAppsMenu(false);
    const rect = menuRefs.current[menuName]?.getBoundingClientRect();
    if (rect) {
      setMenuPosition({ top: rect.bottom + 4, left: rect.left });
      setOpenMenu(openMenu === menuName ? null : menuName);
    }
  }, [openMenu]);

  const closeMenu = useCallback(() => {
    setOpenMenu(null);
  }, []);

  const fileMenuItems: MenuItem[] = [
    { label: 'New Window', icon: '🆕', shortcut: 'Ctrl+N', action: () => console.log('New Window') },
    { label: 'Open File', icon: '📁', shortcut: 'Ctrl+O', action: () => console.log('Open File') },
    { label: 'Save', icon: '💾', shortcut: 'Ctrl+S', action: () => console.log('Save') },
    { divider: true },
    { label: 'Exit', icon: '🚪', shortcut: 'Alt+F4', action: () => window.close() },
  ];

  const editMenuItems: MenuItem[] = [
    { label: 'Copy', icon: '📋', shortcut: 'Ctrl+C', action: () => document.execCommand('copy') },
    { label: 'Paste', icon: '📄', shortcut: 'Ctrl+V', action: () => document.execCommand('paste') },
    { label: 'Cut', icon: '✂️', shortcut: 'Ctrl+X', action: () => document.execCommand('cut') },
    { divider: true },
    { label: 'Preferences', icon: '⚙️', action: () => console.log('Preferences') },
  ];

  const viewMenuItems: MenuItem[] = [
    { label: isDark ? 'Light Mode' : 'Dark Mode', icon: isDark ? '☀️' : '🌙', shortcut: 'Ctrl+T', action: toggleTheme },
    { divider: true },
    { label: 'Icon Size', icon: '📐', action: () => {
      const sizes: Array<"small" | "medium" | "large"> = ["small", "medium", "large"];
      const idx = sizes.indexOf(iconSize);
      setIconSize(sizes[(idx + 1) % sizes.length]);
    }},
    { label: 'Show Desktop', icon: '🖥️', shortcut: 'Ctrl+D', action: minimizeAll },
    { label: 'Refresh', icon: '🔄', shortcut: 'F5', action: () => window.location.reload() },
  ];

  const toolsMenuItems: MenuItem[] = [
    { label: 'All Apps', icon: '🚀', shortcut: 'Ctrl+Shift+L', action: () => navigate('/apps') },
    { label: 'App Store', icon: '📦', shortcut: 'Ctrl+Shift+A', action: () => navigate('/appstore') },
    { divider: true },
    { label: 'Developer Tools', icon: '🛠️', shortcut: 'F12', action: () => console.log('Dev Tools') },
    { label: 'Terminal', icon: '💻', shortcut: 'Ctrl+`', action: () => console.log('Terminal') },
    { label: 'Task Manager', icon: '📊', action: () => console.log('Task Manager') },
    { divider: true },
    { label: 'System Info', icon: 'ℹ️', action: () => console.log('System Info') },
  ];

  const helpMenuItems: MenuItem[] = [
    { label: 'Documentation', icon: '📚', action: () => console.log('Documentation') },
    { label: 'Keyboard Shortcuts', icon: '⌨️', action: () => console.log('Shortcuts') },
    { divider: true },
    { label: 'About X3 Desktop', icon: '🏢', action: () => console.log('About') },
  ];

  const pluginsMenuItems: MenuItem[] = [
    { label: 'Autostart', icon: '🚀', action: () => alert('Tauri Plugin: Autostart triggered') },
    { label: 'Clipboard (Copy Test)', icon: '📋', action: () => {
        navigator.clipboard.writeText('X3 Chain System Info copied!').then(() => alert('Copied to clipboard!'));
    }},
    { label: 'Dialog', icon: '💬', action: () => alert('Tauri Plugin: Dialog triggered') },
    { label: 'Filesystem', icon: '📂', action: () => alert('Tauri Plugin: Filesystem triggered') },
    { label: 'Global Shortcut', icon: '⌨️', action: () => alert('Tauri Plugin: Global Shortcut triggered') },
    { label: 'Log', icon: '📝', action: () => alert('Tauri Plugin: Log triggered') },
    { label: 'Notification', icon: '🔔', action: () => {
        if ("Notification" in window) {
            Notification.requestPermission().then(permission => {
                if (permission === "granted") new Notification("X3 Chain", { body: "System is nominal." });
            });
        } else alert('Native notifications not supported in this environment.');
    }},
    { label: 'Opener', icon: '🔗', action: () => window.open('https://x3-chain.com', '_blank') },
    { label: 'OS Info', icon: '💻', action: () => alert(`OS: ${navigator.platform}\nUserAgent: ${navigator.userAgent}`) },
    { label: 'Process', icon: '⚙️', action: () => alert('Tauri Plugin: Process triggered') },
    { label: 'Screenshot (Mock)', icon: '📸', action: () => {
        alert('Screenshot saved to clipboard! (Simulated)');
    }},
    { label: 'Store', icon: '💾', action: () => alert('Tauri Plugin: Store triggered') },
  ];

  const menuItems = {
    file: fileMenuItems,
    edit: editMenuItems,
    view: viewMenuItems,
    tools: toolsMenuItems,
    plugins: pluginsMenuItems,
    help: helpMenuItems,
  };

  return (
    <div className="relative">
      {/* Top Navigation Bar */}
      <div className="h-10 bg-gradient-to-r from-[#0a0a0c]/95 via-[#0d1f26]/95 to-[#0a0a0c]/95
        backdrop-blur-md border-b border-[#1a9fb5]/30 flex items-center px-4 gap-1 z-30"
        style={{
          boxShadow: '0 4px 20px rgba(26, 159, 181, 0.3), 0 0 30px rgba(26, 159, 181, 0.15), inset 0 1px 0 rgba(26, 159, 181, 0.2)'
        }}>

        {/* X3 Logo */}
        <div className="flex items-center gap-2 mr-6">
          <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#1a9fb5] to-[#0d5f7a]
            flex items-center justify-center text-xs font-bold text-white shadow-lg"
            style={{
              boxShadow: '0 0 15px rgba(26, 159, 181, 0.6)'
            }}>
            A
          </div>
          <span className="text-sm font-semibold text-[#2ab4cc]">X3</span>
        </div>

        {/* Menu Buttons */}
        {Object.keys(menuItems).map((menuName) => (
          <button
            key={menuName}
            ref={(el) => menuRefs.current[menuName] = el}
            className={`px-3 py-1 text-sm font-medium rounded-md transition-all duration-200
              hover:bg-[#1a9fb5]/20 hover:text-[#2ab4cc] capitalize
              ${openMenu === menuName ? 'bg-[#1a9fb5]/30 text-[#2ab4cc] shadow-md' : 'text-[#a8a8a8]'}`}
            onClick={(e) => handleMenuClick(menuName, e)}
          >
            {menuName}
          </button>
        ))}

        {/* Apps Mega-Menu Button */}
        <button
          ref={appsButtonRef}
          className={`px-3 py-1 text-sm font-medium rounded-md transition-all duration-200
            hover:bg-[#1a9fb5]/20 hover:text-[#2ab4cc]
            ${showAppsMenu ? 'bg-[#1a9fb5]/30 text-[#2ab4cc] shadow-md' : 'text-[#a8a8a8]'}`}
          onClick={() => { setOpenMenu(null); setShowAppsMenu(!showAppsMenu); setAppsMenuFilter(''); }}
        >
          🚀 Apps
        </button>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Navigation Links */}
        <div className="flex items-center gap-2 mr-4">
          <button onClick={() => navigate('/social')} className="px-2 py-1 text-xs font-medium text-text-secondary hover:text-accent-primary hover:bg-accent-primary/10 rounded transition-all">
            🌐 Social
          </button>
          <button onClick={() => navigate('/crm')} className="px-2 py-1 text-xs font-medium text-text-secondary hover:text-accent-primary hover:bg-accent-primary/10 rounded transition-all">
            📅 CRM
          </button>
          <button onClick={() => { launch('wallet'); }} className="px-2 py-1 text-xs font-medium text-text-secondary hover:text-accent-primary hover:bg-accent-primary/10 rounded transition-all">
            👛 X3 Wallet
          </button>
          <button onClick={() => navigate('/benchmark-ultimate')} className="px-2 py-1 text-xs font-medium text-text-secondary hover:text-accent-primary hover:bg-accent-primary/10 rounded transition-all">
            ⚡ Full Chain Bench Ultimate
          </button>
          <button onClick={() => { launch('world-monitor'); }} className="px-2 py-1 text-xs font-medium text-[#26c6da] hover:bg-[#26c6da]/10 rounded transition-all">
            🌍 Crypto World Monitor
          </button>
          <button onClick={() => { launch('x3star'); }} className="px-2 py-1 text-xs font-medium text-[#ff9900] hover:bg-[#ff9900]/10 rounded transition-all">
            ⚙️ Exec Engine
          </button>
        </div>

        {/* Network selector */}
        <div className="flex items-center gap-2 mr-4">
          <select
            value={selectedNetwork}
            onChange={e => setSelectedNetwork(e.target.value)}
            className="text-xs bg-bg-primary border border-border-default rounded px-2 py-1 text-text-primary"
            title="Select network"
          >
            {NETWORK_OPTIONS.map(opt => (
              <option key={opt.id} value={opt.id}>{opt.label}</option>
            ))}
          </select>
        </div>

        {/* User / Login */}
        <div className="relative flex items-center gap-2">
          {isLoggedIn && currentUser ? (
            <div className="flex items-center gap-2">
              {currentUser.role && ROLE_ICON[currentUser.role] && (
                <span style={{
                  background: `${ROLE_COLOR[currentUser.role]}22`,
                  border: `1px solid ${ROLE_COLOR[currentUser.role]}55`,
                  color: ROLE_COLOR[currentUser.role],
                  borderRadius: 8, padding: '1px 6px', fontSize: '0.55rem', fontWeight: 700,
                }}>
                  {ROLE_ICON[currentUser.role]}
                </span>
              )}
              <button
                onClick={() => setShowLoginDropdown(!showLoginDropdown)}
                className="flex items-center gap-1 px-2 py-1 text-xs font-medium text-text-primary hover:bg-accent-primary/10 rounded transition-all"
              >
                <div className="w-5 h-5 rounded-full bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center text-[9px] font-bold text-white">
                  {(currentUser.displayName || currentUser.username)[0].toUpperCase()}
                </div>
                <span className="max-w-[80px] truncate">{currentUser.displayName || session?.username}</span>
              </button>
              {showLoginDropdown && (
                <>
                  <div className="fixed inset-0 z-40" onClick={() => setShowLoginDropdown(false)} />
                  <div className="absolute right-0 top-9 z-50 glass-panel border border-border-default rounded-lg shadow-2xl min-w-[180px] py-1">
                    <button onClick={() => { navigate('/social/profile'); setShowLoginDropdown(false); }}
                      className="w-full px-4 py-2 text-left text-sm text-text-primary hover:bg-accent-primary/10 transition-colors flex items-center gap-2">
                      👤 My Profile
                    </button>
                    <button onClick={() => { navigate('/social/messages'); setShowLoginDropdown(false); }}
                      className="w-full px-4 py-2 text-left text-sm text-text-primary hover:bg-accent-primary/10 transition-colors flex items-center gap-2">
                      ✉️ Messages
                    </button>
                    <button onClick={() => { navigate('/crm'); setShowLoginDropdown(false); }}
                      className="w-full px-4 py-2 text-left text-sm text-text-primary hover:bg-accent-primary/10 transition-colors flex items-center gap-2">
                      📅 Calendar CRM
                    </button>
                    <div className="border-t border-border-default my-1" />
                    <button onClick={handleSignOut}
                      className="w-full px-4 py-2 text-left text-sm text-red-400 hover:bg-red-500/10 transition-colors flex items-center gap-2">
                      🚪 Sign Out
                    </button>
                  </div>
                </>
              )}
            </div>
          ) : (
            <div className="relative">
              <button
                onClick={() => setShowLoginDropdown(!showLoginDropdown)}
                className="flex items-center gap-1 px-3 py-1 text-xs font-medium bg-accent-primary/20 text-accent-primary hover:bg-accent-primary/30 rounded-lg transition-all"
              >
                🔑 Sign In
              </button>
              {showLoginDropdown && (
                <>
                  <div className="fixed inset-0 z-40" onClick={() => setShowLoginDropdown(false)} />
                  <div className="absolute right-0 top-9 z-50 glass-panel border border-border-default rounded-lg shadow-2xl min-w-[260px] p-4">
                    <div className="text-sm font-semibold text-text-primary mb-3">AtlasSpace Login</div>
                    {loginError && <div className="text-xs text-red-400 mb-2">{loginError}</div>}
                    <form onSubmit={handleQuickLogin} className="flex flex-col gap-2">
                      <input
                        className="px-3 py-1.5 text-xs bg-bg-primary border border-border-default rounded text-text-primary focus:border-accent-primary outline-none"
                        placeholder="Username"
                        value={loginUser}
                        onChange={e => setLoginUser(e.target.value)}
                        autoFocus
                      />
                      <input
                        className="px-3 py-1.5 text-xs bg-bg-primary border border-border-default rounded text-text-primary focus:border-accent-primary outline-none"
                        type="password"
                        placeholder="Password"
                        value={loginPass}
                        onChange={e => setLoginPass(e.target.value)}
                      />
                      <button
                        type="submit"
                        disabled={loginLoading}
                        className="px-3 py-1.5 text-xs font-medium bg-accent-primary text-white rounded hover:bg-accent-primary/80 transition-colors disabled:opacity-50"
                      >
                        {loginLoading ? 'Signing in...' : 'Sign In'}
                      </button>
                    </form>
                    <div className="mt-2 text-center">
                      <button onClick={() => { navigate('/social'); setShowLoginDropdown(false); }}
                        className="text-[10px] text-accent-primary hover:underline">
                        Create Account / Use Team Code →
                      </button>
                    </div>
                    {universalWallet && (
                      <div className="mt-4 pt-4 border-t border-border-default">
                        <button
                          onClick={handleWalletLogin}
                          disabled={loginLoading}
                          className="w-full flex items-center justify-center gap-2 px-3 py-2 text-xs font-bold bg-gradient-to-r from-orange-500 to-purple-600 text-white rounded-lg hover:from-orange-600 hover:to-purple-700 transition-all shadow-lg shadow-orange-500/20"
                        >
                          👛 Connect with X3 Wallet
                        </button>
                        <p className="text-[9px] text-text-secondary mt-2 text-center">
                          Active: {universalWallet.evm_address.slice(0, 8)}...
                        </p>
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          )}
        </div>

        {/* Status Indicators */}
        <div className="flex items-center gap-3 text-xs text-text-secondary ml-3">
          <div className="flex items-center gap-1">
            <div className={`w-2 h-2 rounded-full ${isLoggedIn ? 'bg-green-500' : 'bg-gray-500'} animate-pulse`}></div>
            <span>{isLoggedIn ? 'Online' : 'Offline'}</span>
          </div>
          <div className="flex items-center gap-1">
            <span>🕒</span>
            <span>{new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
          </div>
        </div>
      </div>

      {/* Dropdown Menus */}
      {Object.keys(menuItems).map((menuName) => (
        <DropdownMenu
          key={menuName}
          items={menuItems[menuName as keyof typeof menuItems]}
          isOpen={openMenu === menuName}
          onClose={closeMenu}
          position={menuPosition}
        />
      ))}

      {/* Apps Mega-Dropdown */}
      {showAppsMenu && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setShowAppsMenu(false)} />
          <div
            className="absolute z-50 glass-panel border border-border-default rounded-xl shadow-2xl"
            style={{
              top: (appsButtonRef.current?.getBoundingClientRect().bottom ?? 40) + 4,
              left: Math.max(8, (appsButtonRef.current?.getBoundingClientRect().left ?? 0) - 60),
              width: 'min(90vw, 820px)',
              maxHeight: '70vh',
            }}
          >
            {/* Search */}
            <div className="p-3 border-b border-border-default">
              <input
                autoFocus
                placeholder="Search apps…"
                value={appsMenuFilter}
                onChange={e => setAppsMenuFilter(e.target.value)}
                className="w-full px-3 py-1.5 text-xs bg-bg-primary border border-border-default rounded-lg text-text-primary focus:border-accent-primary outline-none"
              />
            </div>

            {/* Preinstalled / pinned apps (prominent) */}
            {/** Show preinstalled apps first so they're easy to access */}
            {(() => {
              const preinstalledApps = applications.filter(a => !!a.preinstalled);
              if (preinstalledApps.length === 0) return null;
              return (
                <div className="p-3 border-b border-border-default">
                  <div className="text-[11px] text-text-secondary mb-2 font-semibold">Pinned / Preinstalled</div>
                  <div className="flex gap-2 flex-wrap">
                    {preinstalledApps.map(app => (
                      <button
                        key={app.id}
                        onClick={() => { launch(app.id); setShowAppsMenu(false); }}
                        title={app.description || app.name}
                        className="flex items-center gap-2 px-3 py-2 bg-bg-primary border border-border-default rounded-lg text-xs text-text-primary hover:bg-accent-primary/10 transition"
                      >
                        <span className="w-6 h-6 rounded flex items-center justify-center text-[11px] font-bold text-white"
                          style={{ background: app.icon.color || '#666' }}>{app.name[0]}</span>
                        <span className="truncate max-w-[160px]">{app.name}</span>
                      </button>
                    ))}
                  </div>
                </div>
              );
            })()}

            {/* Categorized grid */}
            <div className="overflow-y-auto p-3" style={{ maxHeight: 'calc(70vh - 56px)' }}>
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
                {CATEGORY_ORDER.map(cat => {
                  const apps = groupedApps[cat];
                  if (!apps || apps.length === 0) return null;
                  const filtered = appsMenuFilter
                    ? apps.filter(a => a.name.toLowerCase().includes(appsMenuFilter.toLowerCase()))
                    : apps;
                  if (filtered.length === 0) return null;
                  return (
                    <div key={cat}>
                      <div className="text-[10px] uppercase tracking-wider text-text-secondary font-bold mb-2 flex items-center gap-1">
                        <span>{CATEGORY_META[cat].icon}</span>
                        <span>{CATEGORY_META[cat].label}</span>
                        <span className="text-text-secondary/50">({filtered.length})</span>
                      </div>
                      <div className="flex flex-col gap-0.5">
                        {filtered.map(app => (
                          <button
                            key={app.id}
                            title={app.description || app.name}
                            className="w-full text-left px-2 py-1.5 text-xs text-text-primary rounded-md
                              hover:bg-accent-primary/15 hover:text-accent-primary transition-colors
                              flex items-center gap-2 group"
                            onClick={() => { launch(app.id); setShowAppsMenu(false); }}
                          >
                            <span className="w-5 h-5 rounded flex-shrink-0 flex items-center justify-center text-[10px] font-bold text-white"
                              style={{ background: app.icon.color || '#666' }}>
                              {app.name[0]}
                            </span>
                            <span className="truncate">{app.name}</span>
                          </button>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
              {appsMenuFilter && Object.values(groupedApps).every(apps => !apps?.some(a => a.name.toLowerCase().includes(appsMenuFilter.toLowerCase()))) && (
                <div className="text-center text-xs text-text-secondary py-6">No apps match "{appsMenuFilter}"</div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default TopNavBar;