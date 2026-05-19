import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import './AppBar.css';

interface AppBarProps {
  title?: string;
}

/**
 * AppBar Component
 * Displays app title, user info, and logout option
 */
export const AppBar: React.FC<AppBarProps> = ({ title = 'X3 Chain' }) => {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [showUserMenu, setShowUserMenu] = useState(false);

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleProfileClick = () => {
    navigate('/profile');
    setShowUserMenu(false);
  };

  return (
    <header className="app-bar">
      <div className="app-bar-container">
        {/* Logo/Title */}
        <div className="app-bar-title">
          <span className="app-bar-icon">⚡</span>
          <h1>{title}</h1>
        </div>

        {/* User Section */}
        <div className="app-bar-user">
          <div className="user-info">
            <div className="user-avatar">
              {user?.username?.charAt(0).toUpperCase() || 'A'}
            </div>
            <div className="user-details">
              <div className="user-name">{user?.username || 'User'}</div>
              <div className="user-status">Online</div>
            </div>
          </div>

          {/* User Menu */}
          <button
            className="user-menu-button"
            onClick={() => setShowUserMenu(!showUserMenu)}
            aria-expanded={showUserMenu}
            aria-label="User menu"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>

          {/* Dropdown Menu */}
          {showUserMenu && (
            <div className="user-dropdown">
              <button className="dropdown-item" onClick={handleProfileClick}>
                <span className="dropdown-icon">👤</span> Profile Settings
              </button>
              <button
                className="dropdown-item change-password"
                onClick={() => {
                  navigate('/change-password');
                  setShowUserMenu(false);
                }}
              >
                <span className="dropdown-icon">🔐</span> Change Password
              </button>
              <div className="dropdown-divider"></div>
              <button className="dropdown-item logout" onClick={handleLogout}>
                <span className="dropdown-icon">🚪</span> Logout
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Bottom border */}
      <div className="app-bar-border"></div>
    </header>
  );
};

export default AppBar;
