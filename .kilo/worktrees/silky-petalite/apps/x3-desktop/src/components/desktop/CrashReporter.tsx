import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * Crash Report interfaces
 */
export interface SystemInfo {
  os: string;
  arch: string;
  appVersion: string;
  rustVersion: string;
  totalMemory: number;
  freeMemory: number;
  gpuModel?: string;
}

export interface CrashLog {
  id: string;
  timestamp: number;
  errorType: string;
  errorMessage: string;
  stackTrace: string;
  systemInfo: SystemInfo;
  breadcrumbs: CrashBreadcrumb[];
  userNotes?: string;
  attachments: string[]; // file paths
  reportedAt?: number;
  isUploaded: boolean;
}

export interface CrashBreadcrumb {
  timestamp: number;
  action: string;
  details: Record<string, any>;
  severity: 'info' | 'warning' | 'error';
}

export interface CrashReportSettings {
  autoReport: boolean;
  collectSystemInfo: boolean;
  collectBreadcrumbs: boolean;
  storageQuota: number; // MB
  maxStoredReports: number;
  uploadEndpoint: string;
}

/**
 * CrashReporter Component
 * Automatically captures application crashes, collects system info,
 * and provides one-click bug report submission.
 */
const CrashReporter: React.FC = () => {
  const [crashes, setCrashes] = useState<CrashLog[]>([]);
  const [selectedCrash, setSelectedCrash] = useState<CrashLog | null>(null);
  const [settings, setSettings] = useState<CrashReportSettings>({
    autoReport: true,
    collectSystemInfo: true,
    collectBreadcrumbs: true,
    storageQuota: 100,
    maxStoredReports: 50,
    uploadEndpoint: 'https://api.x3.dev/crash-reports'
  });
  const [isLoading, setIsLoading] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [uploadStatus, setUploadStatus] = useState<string>('');
  const [breadcrumbs, setBreadcrumbs] = useState<CrashBreadcrumb[]>([]);
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);

  // Initialize crash reporter and set up error listeners
  useEffect(() => {
    initializeCrashReporter();
    setupGlobalErrorHandlers();
    loadStoredCrashes();
  }, []);

  /**
   * Initialize crash reporter with Tauri integration
   */
  const initializeCrashReporter = async () => {
    try {
      const storedSettings = await invoke<CrashReportSettings>(
        'get_crash_reporter_settings'
      );
      setSettings(storedSettings);

      // Register Tauri crash hook
      await invoke('setup_crash_hook');
    } catch (error) {
      console.error('Failed to initialize crash reporter:', error);
    }
  };

  /**
   * Set up global error handlers for JavaScript errors
   */
  const setupGlobalErrorHandlers = () => {
    // Handle uncaught errors
    const handleError = (event: ErrorEvent) => {
      captureCrash({
        errorType: event.error?.name || 'Error',
        errorMessage: event.message,
        stackTrace: event.error?.stack || ''
      });
    };

    // Handle unhandled promise rejections
    const handleRejection = (event: PromiseRejectionEvent) => {
      captureCrash({
        errorType: 'UnhandledRejection',
        errorMessage: String(event.reason),
        stackTrace: event.reason?.stack || ''
      });
    };

    window.addEventListener('error', handleError);
    window.addEventListener('unhandledrejection', handleRejection);

    return () => {
      window.removeEventListener('error', handleError);
      window.removeEventListener('unhandledrejection', handleRejection);
    };
  };

  /**
   * Capture crash with system info and breadcrumbs
   */
  const captureCrash = async (crash: Partial<CrashLog>) => {
    try {
      setIsLoading(true);

      // Collect system info
      let systemInfo: SystemInfo | null = null;
      if (settings.collectSystemInfo) {
        systemInfo = await invoke<SystemInfo>('get_system_info');
      }

      const newCrash: CrashLog = {
        id: `crash_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        timestamp: Date.now(),
        errorType: crash.errorType || 'Unknown',
        errorMessage: crash.errorMessage || '',
        stackTrace: crash.stackTrace || '',
        systemInfo: systemInfo || {
          os: 'Unknown',
          arch: 'Unknown',
          appVersion: '1.0.0',
          rustVersion: 'Unknown',
          totalMemory: 0,
          freeMemory: 0
        },
        breadcrumbs: settings.collectBreadcrumbs ? breadcrumbs : [],
        attachments: [],
        isUploaded: false
      };

      // Store crash
      setCrashes(prev => [newCrash, ...prev].slice(0, settings.maxStoredReports));
      await storeCrash(newCrash);

      // Auto-report if enabled
      if (settings.autoReport) {
        await submitCrashReport(newCrash);
      }
    } catch (error) {
      console.error('Failed to capture crash:', error);
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Add breadcrumb for crash investigation
   */
  const addBreadcrumb = (action: string, details: Record<string, any> = {}, severity: 'info' | 'warning' | 'error' = 'info') => {
    const crumb: CrashBreadcrumb = {
      timestamp: Date.now(),
      action,
      details,
      severity
    };
    setBreadcrumbs(prev => [...prev.slice(-99), crumb]); // Keep last 100
  };

  /**
   * Store crash report locally
   */
  const storeCrash = async (crash: CrashLog) => {
    try {
      await invoke('store_crash_report', { crash });
    } catch (error) {
      console.error('Failed to store crash:', error);
    }
  };

  /**
   * Load stored crashes from disk
   */
  const loadStoredCrashes = async () => {
    try {
      setIsLoading(true);
      const storedCrashes = await invoke<CrashLog[]>('load_crash_reports');
      setCrashes(storedCrashes);
    } catch (error) {
      console.error('Failed to load crashes:', error);
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Submit crash report to server
   */
  const submitCrashReport = async (crash: CrashLog) => {
    try {
      setUploadStatus('Uploading...');

      // Prepare form data with attachments
      const formData = new FormData();
      formData.append('report', JSON.stringify(crash));
      selectedFiles.forEach(file => {
        formData.append('attachments', file);
      });

      const response = await fetch(settings.uploadEndpoint, {
        method: 'POST',
        body: formData,
        headers: {
          'X-App-Version': crash.systemInfo.appVersion,
          'X-Error-Type': crash.errorType
        }
      });

      if (response.ok) {
        // Mark as uploaded
        const updated = crashes.map(c =>
          c.id === crash.id ? { ...c, isUploaded: true, reportedAt: Date.now() } : c
        );
        setCrashes(updated);
        setUploadStatus('Report submitted successfully!');
        setSelectedFiles([]);

        // Invoke Tauri to store upload status
        await invoke('update_crash_report_status', {
          crashId: crash.id,
          isUploaded: true
        });

        setTimeout(() => setUploadStatus(''), 3000);
      } else {
        setUploadStatus('Failed to submit report. Retrying...');
      }
    } catch (error) {
      console.error('Failed to submit crash report:', error);
      setUploadStatus('Network error. Report saved locally.');
    }
  };

  /**
   * Delete crash report
   */
  const deleteCrash = async (crashId: string) => {
    try {
      await invoke('delete_crash_report', { crashId });
      setCrashes(prev => prev.filter(c => c.id !== crashId));
      if (selectedCrash?.id === crashId) {
        setSelectedCrash(null);
      }
    } catch (error) {
      console.error('Failed to delete crash:', error);
    }
  };

  /**
   * Bulk delete unuploaded crashes
   */
  const deleteUnuploadedCrashes = async () => {
    try {
      const unuploadedIds = crashes
        .filter(c => !c.isUploaded)
        .map(c => c.id);

      for (const id of unuploadedIds) {
        await deleteCrash(id);
      }
    } catch (error) {
      console.error('Failed to delete crashes:', error);
    }
  };

  /**
   * Handle file attachment
   */
  const handleFileAttachment = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (event.target.files) {
      setSelectedFiles(Array.from(event.target.files));
    }
  };

  /**
   * Export crash report as JSON
   */
  const exportCrashReport = (crash: CrashLog) => {
    const json = JSON.stringify(crash, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `crash_${crash.id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h2>🐛 Crash Reporter</h2>
        <div style={styles.headerButtons}>
          <button
            onClick={() => setShowSettings(!showSettings)}
            style={styles.button}
          >
            ⚙️ Settings
          </button>
          <button
            onClick={loadStoredCrashes}
            style={styles.button}
            disabled={isLoading}
          >
            🔄 Refresh
          </button>
        </div>
      </div>

      {showSettings && (
        <div style={styles.settingsPanel}>
          <label style={styles.label}>
            <input
              type="checkbox"
              checked={settings.autoReport}
              onChange={(e) => setSettings(prev => ({ ...prev, autoReport: e.target.checked }))}
            />
            Auto-report crashes
          </label>
          <label style={styles.label}>
            <input
              type="checkbox"
              checked={settings.collectSystemInfo}
              onChange={(e) => setSettings(prev => ({ ...prev, collectSystemInfo: e.target.checked }))}
            />
            Collect system information
          </label>
          <label style={styles.label}>
            <input
              type="checkbox"
              checked={settings.collectBreadcrumbs}
              onChange={(e) => setSettings(prev => ({ ...prev, collectBreadcrumbs: e.target.checked }))}
            />
            Collect user actions
          </label>
          <label style={styles.label}>
            Max reports to store:
            <input
              type="number"
              min="5"
              max="100"
              value={settings.maxStoredReports}
              onChange={(e) => setSettings(prev => ({ ...prev, maxStoredReports: parseInt(e.target.value) }))}
              style={styles.input}
            />
          </label>
        </div>
      )}

      {uploadStatus && (
        <div style={Object.assign({}, styles.status, {
          color: uploadStatus.includes('success') ? '#10b981' : '#f59e0b'
        })}>
          {uploadStatus}
        </div>
      )}

      <div style={styles.content}>
        <div style={styles.crashList}>
          <h3>Reports ({crashes.length})</h3>
          {isLoading ? (
            <p style={styles.placeholder}>Loading...</p>
          ) : crashes.length === 0 ? (
            <p style={styles.placeholder}>No crashes recorded</p>
          ) : (
            <div>
              {crashes.map(crash => (
                <div
                  key={crash.id}
                  onClick={() => setSelectedCrash(crash)}
                  style={Object.assign({}, styles.crashItem, {
                    backgroundColor: selectedCrash?.id === crash.id ? '#374151' : '#1f2937',
                    borderLeft: crash.isUploaded ? '4px solid #10b981' : '4px solid #f59e0b'
                  })}
                >
                  <div style={styles.crashItemHeader}>
                    <span style={styles.errorType}>{crash.errorType}</span>
                    {crash.isUploaded && <span style={styles.badge}>✓ Uploaded</span>}
                  </div>
                  <p style={styles.crashItemMessage}>{crash.errorMessage.substring(0, 60)}...</p>
                  <p style={styles.crashItemTime}>
                    {new Date(crash.timestamp).toLocaleString()}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>

        {selectedCrash && (
          <div style={styles.details}>
            <h3>Crash Details</h3>
            <div style={styles.detailsContent}>
              <div style={styles.section}>
                <h4>Error</h4>
                <p><strong>Type:</strong> {selectedCrash.errorType}</p>
                <p><strong>Message:</strong> {selectedCrash.errorMessage}</p>
                <details>
                  <summary>Stack Trace</summary>
                  <pre style={styles.stackTrace}>{selectedCrash.stackTrace}</pre>
                </details>
              </div>

              <div style={styles.section}>
                <h4>System Info</h4>
                <p><strong>OS:</strong> {selectedCrash.systemInfo.os}</p>
                <p><strong>Architecture:</strong> {selectedCrash.systemInfo.arch}</p>
                <p><strong>RAM:</strong> {(selectedCrash.systemInfo.freeMemory / 1024 / 1024).toFixed(2)} MB free</p>
              </div>

              {selectedCrash.breadcrumbs.length > 0 && (
                <div style={styles.section}>
                  <h4>User Actions</h4>
                  <div style={styles.breadcrumbs}>
                    {selectedCrash.breadcrumbs.slice(-10).map((crumb, i) => (
                      <div key={i} style={styles.breadcrumb}>
                        <span style={styles.breadcrumbTime}>
                          {new Date(crumb.timestamp).toLocaleTimeString()}
                        </span>
                        <span style={styles.breadcrumbAction}>{crumb.action}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div style={styles.actions}>
                <input
                  type="text"
                  placeholder="Add your notes..."
                  defaultValue={selectedCrash.userNotes}
                  style={styles.notesInput}
                  onChange={(e) => {
                    const updated = crashes.map(c =>
                      c.id === selectedCrash.id
                        ? { ...c, userNotes: e.target.value }
                        : c
                    );
                    setCrashes(updated);
                    setSelectedCrash(updated.find(c => c.id === selectedCrash.id) || null);
                  }}
                />

                <label style={styles.fileInput}>
                  📎 Attach Files
                  <input
                    type="file"
                    multiple
                    onChange={handleFileAttachment}
                    style={{ display: 'none' }}
                  />
                </label>

                {selectedFiles.length > 0 && (
                  <p style={styles.fileCount}>{selectedFiles.length} file(s) selected</p>
                )}

                <div style={styles.buttonGroup}>
                  {!selectedCrash.isUploaded && (
                    <button
                      onClick={() => submitCrashReport(selectedCrash)}
                      style={Object.assign({}, styles.button, styles.submitButton)}
                    >
                      📤 Submit Report
                    </button>
                  )}
                  <button
                    onClick={() => exportCrashReport(selectedCrash)}
                    style={styles.button}
                  >
                    💾 Export JSON
                  </button>
                  <button
                    onClick={() => deleteCrash(selectedCrash.id)}
                    style={Object.assign({}, styles.button, styles.deleteButton)}
                  >
                    🗑️ Delete
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {crashes.filter(c => !c.isUploaded).length > 0 && (
        <div style={styles.footer}>
          <button
            onClick={deleteUnuploadedCrashes}
            style={styles.clearButton}
          >
            Clear unuploaded reports
          </button>
        </div>
      )}
    </div>
  );
};

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    backgroundColor: '#111827',
    color: '#f3f4f6',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '20px',
    borderBottom: '1px solid #1f2937',
    backgroundColor: '#0f172a'
  },
  headerButtons: {
    display: 'flex',
    gap: '10px'
  },
  settingsPanel: {
    padding: '15px 20px',
    backgroundColor: '#1f2937',
    borderBottom: '1px solid #374151',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '10px'
  },
  label: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    cursor: 'pointer'
  },
  input: {
    marginLeft: '10px',
    padding: '4px 8px',
    backgroundColor: '#111827',
    color: '#f3f4f6',
    border: '1px solid #374151',
    borderRadius: '4px'
  },
  status: {
    padding: '12px 20px',
    backgroundColor: '#eff6ff',
    borderBottom: '1px solid #e5e7eb'
  },
  content: {
    display: 'flex',
    flex: 1,
    overflow: 'hidden',
    gap: '20px',
    padding: '20px',
  },
  crashList: {
    flex: '0 0 350px',
    overflowY: 'auto' as const,
    borderRight: '1px solid #1f2937'
  },
  crashItem: {
    padding: '12px',
    marginBottom: '8px',
    backgroundColor: '#1f2937',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
    transition: 'all 0.2s'
  },
  crashItemHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '8px'
  },
  errorType: {
    fontSize: '12px',
    fontWeight: 'bold' as const,
    color: '#fca5a5'
  },
  badge: {
    fontSize: '11px',
    backgroundColor: '#10b981',
    color: '#fff',
    padding: '2px 6px',
    borderRadius: '3px'
  },
  crashItemMessage: {
    fontSize: '13px',
    color: '#d1d5db',
    margin: '4px 0',
    whiteSpace: 'nowrap' as const,
    overflow: 'hidden',
    textOverflow: 'ellipsis'
  },
  crashItemTime: {
    fontSize: '11px',
    color: '#9ca3af'
  },
  details: {
    flex: 1,
    overflow: 'auto' as const,
    backgroundColor: '#1f2937',
    borderRadius: '6px',
    padding: '20px'
  },
  detailsContent: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '20px'
  },
  section: {
    borderBottom: '1px solid #374151',
    paddingBottom: '15px'
  },
  stackTrace: {
    backgroundColor: '#111827',
    padding: '10px',
    borderRadius: '4px',
    fontSize: '11px',
    fontFamily: 'monospace',
    overflow: 'auto' as const,
    color: '#9ca3af',
    maxHeight: '150px'
  },
  breadcrumbs: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '6px',
    fontSize: '12px'
  },
  breadcrumb: {
    display: 'flex',
    gap: '10px',
    padding: '6px',
    backgroundColor: '#111827',
    borderRadius: '4px'
  },
  breadcrumbTime: {
    color: '#6b7280',
    minWidth: '60px'
  },
  breadcrumbAction: {
    color: '#d1d5db'
  },
  actions: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '10px'
  },
  notesInput: {
    padding: '10px',
    backgroundColor: '#111827',
    color: '#f3f4f6',
    border: '1px solid #374151',
    borderRadius: '4px',
    fontSize: '13px'
  },
  fileInput: {
    padding: '10px',
    backgroundColor: '#374151',
    border: '1px dashed #6b7280',
    borderRadius: '4px',
    cursor: 'pointer',
    textAlign: 'center' as const,
    fontSize: '13px'
  },
  fileCount: {
    fontSize: '12px',
    color: '#10b981'
  },
  buttonGroup: {
    display: 'flex',
    gap: '10px',
    justifyContent: 'flex-end'
  },
  button: {
    padding: '8px 16px',
    backgroundColor: '#374151',
    color: '#f3f4f6',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '13px',
    transition: 'all 0.2s'
  },
  submitButton: {
    backgroundColor: '#059669'
  },
  deleteButton: {
    backgroundColor: '#dc2626'
  },
  footer: {
    padding: '15px 20px',
    borderTop: '1px solid #1f2937',
    display: 'flex',
    justifyContent: 'flex-end'
  },
  clearButton: {
    padding: '8px 16px',
    backgroundColor: '#6b7280',
    color: '#fff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '13px'
  },
  placeholder: {
    color: '#6b7280',
    textAlign: 'center' as const,
    padding: '20px'
  }
};

export default CrashReporter;
