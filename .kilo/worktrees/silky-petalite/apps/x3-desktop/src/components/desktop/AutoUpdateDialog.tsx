// Auto-update dialog with changelog display
// Notifies users of new versions and allows upgrade/deferral

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';

export interface UpdateVersion {
  version: string;
  release_date: Date;
  download_size: number;
  breaking_changes: string[];
  new_features: string[];
  bug_fixes: string[];
  security_updates: string[];
}

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  available_update: boolean;
  update_url: string;
  changelog: string;
  versions: UpdateVersion[];
  download_progress?: number;
  is_downloading: boolean;
  critical: boolean;
}

export const AutoUpdateDialog: React.FC = () => {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [showDialog, setShowDialog] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [deferred, setDeferred] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [selectedTab, setSelectedTab] = useState<'summary' | 'changelog' | 'details'>('summary');

  // Check for updates on mount
  useEffect(() => {
    const checkForUpdates = async () => {
      try {
        const info = await invoke<UpdateInfo>('check_for_updates');
        setUpdateInfo(info);

        if (info.available_update && (info.critical || !deferred)) {
          setShowDialog(true);
        }
      } catch (error) {
        console.error('Failed to check for updates:', error);
      }
    };

    checkForUpdates();

    // Check every hour for updates
    const interval = setInterval(checkForUpdates, 3600000);
    return () => clearInterval(interval);
  }, [deferred]);

  // Listen for download progress
  useEffect(() => {
    let interval: NodeJS.Timeout;

    if (updateInfo?.is_downloading) {
      interval = setInterval(async () => {
        try {
          const progress = await invoke<number>('get_download_progress');
          setDownloadProgress(progress);
        } catch (error) {
          console.error('Failed to get download progress:', error);
        }
      }, 500);
    }

    return () => clearInterval(interval);
  }, [updateInfo?.is_downloading]);

  // Install update
  const installUpdate = async () => {
    if (!updateInfo) return;

    setInstalling(true);
    try {
      await invoke('install_update', { version: updateInfo.latest_version });

      // Relaunch app after update
      setTimeout(() => {
        relaunch();
      }, 2000);
    } catch (error) {
      console.error('Failed to install update:', error);
      setInstalling(false);
    }
  };

  // Defer update
  const deferUpdate = () => {
    setDeferred(true);
    setShowDialog(false);

    // Re-prompt in 24 hours
    setTimeout(() => {
      setDeferred(false);
    }, 86400000);
  };

  if (!updateInfo || !updateInfo.available_update || !showDialog) {
    return null;
  }

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
  };

  const getLatestVersion = (): UpdateVersion | undefined => {
    return updateInfo.versions.find(v => v.version === updateInfo.latest_version);
  };

  const latestVersion = getLatestVersion();

  return (
    <div className="auto-update-overlay">
      <div className={`update-dialog ${updateInfo.critical ? 'critical' : ''}`}>
        <div className="dialog-header">
          <div className="header-content">
            <h2>
              {updateInfo.critical ? '🚨 Critical Update' : '📦 Update Available'}
            </h2>
            <p className="version-info">
              v{updateInfo.current_version} → v{updateInfo.latest_version}
            </p>
          </div>
          {!installing && !updateInfo.is_downloading && (
            <button
              className="btn-close"
              onClick={() => setShowDialog(false)}
              aria-label="Close"
            >
              ✕
            </button>
          )}
        </div>

        {/* Tab Navigation */}
        <div className="tab-navigation">
          <button
            className={`tab-btn ${selectedTab === 'summary' ? 'active' : ''}`}
            onClick={() => setSelectedTab('summary')}
          >
            Summary
          </button>
          <button
            className={`tab-btn ${selectedTab === 'changelog' ? 'active' : ''}`}
            onClick={() => setSelectedTab('changelog')}
          >
            Changes
          </button>
          <button
            className={`tab-btn ${selectedTab === 'details' ? 'active' : ''}`}
            onClick={() => setSelectedTab('details')}
          >
            Details
          </button>
        </div>

        {/* Content Area */}
        <div className="dialog-content">
          {selectedTab === 'summary' && (
            <div className="tab-content">
              <div className="summary-section">
                <h3>What's New?</h3>
                {latestVersion ? (
                  <div className="changes-list">
                    {latestVersion.new_features.length > 0 && (
                      <div className="change-group">
                        <h4>✨ New Features</h4>
                        <ul>
                          {latestVersion.new_features.map((feature, idx) => (
                            <li key={idx}>{feature}</li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {latestVersion.bug_fixes.length > 0 && (
                      <div className="change-group">
                        <h4>🐛 Bug Fixes</h4>
                        <ul>
                          {latestVersion.bug_fixes.map((fix, idx) => (
                            <li key={idx}>{fix}</li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {latestVersion.security_updates.length > 0 && (
                      <div className="change-group security">
                        <h4>🔒 Security Updates</h4>
                        <ul>
                          {latestVersion.security_updates.map((update, idx) => (
                            <li key={idx}>{update}</li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {latestVersion.breaking_changes.length > 0 && (
                      <div className="change-group warning">
                        <h4>⚠️ Breaking Changes</h4>
                        <ul>
                          {latestVersion.breaking_changes.map((change, idx) => (
                            <li key={idx}>{change}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                ) : (
                  <p>No version details available.</p>
                )}
              </div>

              {updateInfo.critical && (
                <div className="critical-warning">
                  <h4>⚠️ Critical Update</h4>
                  <p>
                    This is a critical security or stability update. We recommend installing
                    it immediately.
                  </p>
                </div>
              )}
            </div>
          )}

          {selectedTab === 'changelog' && (
            <div className="tab-content">
              <div className="changelog">
                <div className="changelog-text">{updateInfo.changelog}</div>
              </div>
            </div>
          )}

          {selectedTab === 'details' && (
            <div className="tab-content">
              <div className="details-grid">
                <div className="detail-item">
                  <label>Current Version</label>
                  <span>v{updateInfo.current_version}</span>
                </div>
                <div className="detail-item">
                  <label>Latest Version</label>
                  <span>v{updateInfo.latest_version}</span>
                </div>
                {latestVersion && (
                  <>
                    <div className="detail-item">
                      <label>Release Date</label>
                      <span>{new Date(latestVersion.release_date).toLocaleDateString()}</span>
                    </div>
                    <div className="detail-item">
                      <label>Download Size</label>
                      <span>{formatBytes(latestVersion.download_size)}</span>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Progress Bar */}
        {(updateInfo.is_downloading || installing) && (
          <div className="progress-section">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${downloadProgress}%` }}
              ></div>
            </div>
            <p className="progress-text">
              {installing ? 'Installing' : 'Downloading'} update... {downloadProgress}%
            </p>
          </div>
        )}

        {/* Action Buttons */}
        <div className="dialog-actions">
          {!installing && !updateInfo.is_downloading ? (
            <>
              {!updateInfo.critical && (
                <button
                  className="btn-defer"
                  onClick={deferUpdate}
                >
                  Remind Later
                </button>
              )}
              <button
                className="btn-install"
                onClick={installUpdate}
              >
                Install Update
              </button>
            </>
          ) : (
            <p className="installing-message">
              {installing
                ? 'Installation in progress. This may take a few moments...'
                : 'Download in progress. Please wait...'}
            </p>
          )}
        </div>

        {/* Note */}
        <div className="dialog-note">
          <p>
            💡 The app will restart automatically once the update is installed. Make sure to save
            any unsaved work.
          </p>
        </div>
      </div>
    </div>
  );
};

export default AutoUpdateDialog;

const styles = `
  .auto-update-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10001;
    padding: 20px;
  }

  .update-dialog {
    background: white;
    border-radius: 12px;
    max-width: 600px;
    width: 100%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }

  .update-dialog.critical {
    border: 2px solid #dc3545;
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: start;
    padding: 24px;
    border-bottom: 1px solid #eee;
  }

  .header-content h2 {
    margin: 0 0 8px 0;
    font-size: 20px;
  }

  .version-info {
    margin: 0;
    font-size: 14px;
    color: #666;
  }

  .btn-close {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: #666;
    padding: 0;
    transition: color 0.2s;
  }

  .btn-close:hover {
    color: #333;
  }

  .tab-navigation {
    display: flex;
    border-bottom: 1px solid #eee;
    padding: 0 24px;
    gap: 24px;
  }

  .tab-btn {
    background: none;
    border: none;
    padding: 12px 0;
    font-size: 14px;
    cursor: pointer;
    color: #666;
    border-bottom: 3px solid transparent;
    transition: all 0.2s;
  }

  .tab-btn:hover {
    color: #333;
  }

  .tab-btn.active {
    color: #0066cc;
    border-bottom-color: #0066cc;
  }

  .dialog-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .summary-section h3 {
    margin: 0 0 16px 0;
    font-size: 16px;
  }

  .changes-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .change-group {
    background: #f9f9f9;
    padding: 12px;
    border-radius: 6px;
  }

  .change-group h4 {
    margin: 0 0 8px 0;
    font-size: 13px;
    font-weight: 600;
  }

  .change-group ul {
    margin: 0;
    padding-left: 20px;
  }

  .change-group li {
    font-size: 13px;
    color: #333;
    margin-bottom: 4px;
  }

  .change-group.security {
    background: #d4edda;
    border-left: 3px solid #28a745;
  }

  .change-group.warning {
    background: #fff3cd;
    border-left: 3px solid #ffc107;
  }

  .critical-warning {
    background: #f8d7da;
    border: 1px solid #f5c6cb;
    border-radius: 6px;
    padding: 12px;
    margin-top: 12px;
  }

  .critical-warning h4 {
    margin: 0 0 6px 0;
    color: #721c24;
    font-size: 13px;
  }

  .critical-warning p {
    margin: 0;
    color: #721c24;
    font-size: 12px;
  }

  .changelog {
    background: #f9f9f9;
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 16px;
    font-family: monospace;
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .detail-item {
    background: #f9f9f9;
    padding: 12px;
    border-radius: 6px;
  }

  .detail-item label {
    display: block;
    font-size: 11px;
    color: #666;
    margin-bottom: 4px;
    text-transform: uppercase;
    font-weight: 600;
  }

  .detail-item value {
    display: block;
    font-size: 14px;
    color: #333;
    font-weight: 500;
  }

  .progress-section {
    padding: 24px;
    border-top: 1px solid #eee;
  }

  .progress-bar {
    background: #e0e0e0;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #0066cc, #0052a3);
    transition: width 0.3s ease;
  }

  .progress-text {
    margin: 0;
    font-size: 12px;
    color: #666;
    text-align: center;
  }

  .dialog-actions {
    display: flex;
    gap: 12px;
    padding: 24px;
    border-top: 1px solid #eee;
  }

  .btn-defer,
  .btn-install {
    flex: 1;
    padding: 12px 24px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-defer {
    background: #f0f0f0;
    color: #333;
  }

  .btn-defer:hover {
    background: #e0e0e0;
  }

  .btn-install {
    background: #0066cc;
    color: white;
  }

  .btn-install:hover {
    background: #0052a3;
  }

  .installing-message {
    margin: 0;
    font-size: 13px;
    color: #666;
    text-align: center;
  }

  .dialog-note {
    background: #e7f3ff;
    border: 1px solid #b3d9ff;
    padding: 12px;
    margin: 0;
    border-radius: 0 0 12px 12px;
  }

  .dialog-note p {
    margin: 0;
    font-size: 12px;
    color: #004085;
  }
`;
