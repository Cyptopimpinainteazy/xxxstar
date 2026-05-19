// System notifications for transactions, alerts, messages, and prices
// Uses native Tauri notifications

import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export enum NotificationCategory {
  Transaction = 'transaction',
  Alert = 'alert',
  Message = 'message',
  Price = 'price',
  Validator = 'validator',
  System = 'system',
}

export enum NotificationPriority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical',
}

export interface Notification {
  id: string;
  title: string;
  body: string;
  category: NotificationCategory;
  priority: NotificationPriority;
  icon?: string;
  sound: boolean;
  timestamp: Date;
  read: boolean;
  action_url?: string;
  action_label?: string;
}

interface NotificationState {
  notifications: Notification[];
  unread_count: number;
  settings: NotificationSettings;
  history: Notification[];
}

interface NotificationSettings {
  enabled: boolean;
  sound_enabled: boolean;
  desktop_enabled: boolean;
  category_settings: Map<NotificationCategory, boolean>;
  do_not_disturb: boolean;
  dnd_start: string; // HH:MM format
  dnd_end: string;   // HH:MM format
}

export const SystemNotifications: React.FC = () => {
  const [state, setState] = useState<NotificationState>({
    notifications: [],
    unread_count: 0,
    settings: {
      enabled: true,
      sound_enabled: true,
      desktop_enabled: true,
      category_settings: new Map([
        [NotificationCategory.Transaction, true],
        [NotificationCategory.Alert, true],
        [NotificationCategory.Message, true],
        [NotificationCategory.Price, false], // Disabled by default (too noisy)
        [NotificationCategory.Validator, true],
        [NotificationCategory.System, true],
      ]),
      do_not_disturb: false,
      dnd_start: '22:00',
      dnd_end: '08:00',
    },
    history: [],
  });

  const [showHistory, setShowHistory] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<NotificationCategory | 'all'>('all');

  // Listen for notification events
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const setupListeners = async () => {
      // Listen to different notification events
      const eventTypes = [
        'tx-confirmed',
        'tx-failed',
        'alert-triggered',
        'message-received',
        'price-alert',
        'validator-alert',
      ];

      for (const eventType of eventTypes) {
        try {
          const unlisten = await listen(eventType, (event: any) => {
            handleNotificationEvent(eventType, event.payload);
          });
          unlisteners.push(unlisten);
        } catch (error) {
          console.error(`Failed to listen to ${eventType}:`, error);
        }
      }
    };

    setupListeners();

    return () => {
      unlisteners.forEach(unlisten => unlisten());
    };
  }, []);

  // Load notification history on mount
  useEffect(() => {
    const loadHistory = async () => {
      try {
        const history = await invoke<Notification[]>('get_notification_history');
        setState(prev => ({
          ...prev,
          history: history,
        }));
      } catch (error) {
        console.error('Failed to load notification history:', error);
      }
    };

    loadHistory();
  }, []);

  // Check if notifications should be suppressed (DND mode)
  const isInDoNotDisturb = useCallback((): boolean => {
    if (!state.settings.do_not_disturb) return false;

    const now = new Date();
    const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

    const start = state.settings.dnd_start;
    const end = state.settings.dnd_end;

    // Handle overnight case (e.g., 22:00 to 08:00)
    if (start > end) {
      return currentTime >= start || currentTime < end;
    }

    return currentTime >= start && currentTime < end;
  }, [state.settings.do_not_disturb, state.settings.dnd_start, state.settings.dnd_end]);

  // Handle incoming notification event
  const handleNotificationEvent = useCallback((eventType: string, payload: any) => {
    const categoryMap: Record<string, NotificationCategory> = {
      'tx-confirmed': NotificationCategory.Transaction,
      'tx-failed': NotificationCategory.Transaction,
      'alert-triggered': NotificationCategory.Alert,
      'message-received': NotificationCategory.Message,
      'price-alert': NotificationCategory.Price,
      'validator-alert': NotificationCategory.Validator,
    };

    const category = categoryMap[eventType];
    if (!category) return;

    // Check if category is enabled
    if (!state.settings.category_settings.get(category)) return;

    const notification: Notification = {
      id: `${Date.now()}-${Math.random()}`,
      title: payload.title || 'Notification',
      body: payload.body || '',
      category,
      priority: payload.priority || NotificationPriority.Normal,
      icon: payload.icon,
      sound: state.settings.sound_enabled && !isInDoNotDisturb(),
      timestamp: new Date(),
      read: false,
      action_url: payload.action_url,
      action_label: payload.action_label,
    };

    // Show system notification if enabled
    if (state.settings.desktop_enabled && !isInDoNotDisturb()) {
      showSystemNotification(notification);
    }

    // Play sound if enabled
    if (notification.sound) {
      playNotificationSound(category);
    }

    // Update state
    setState(prev => ({
      ...prev,
      notifications: [notification, ...prev.notifications].slice(0, 20), // Keep last 20
      unread_count: prev.unread_count + 1,
      history: [notification, ...prev.history],
    }));
  }, [state.settings, isInDoNotDisturb]);

  // Show native system notification
  const showSystemNotification = async (notification: Notification) => {
    try {
      await invoke('show_notification', {
        title: notification.title,
        body: notification.body,
        icon: notification.icon || getCategoryIcon(notification.category),
        sound: notification.sound,
        category: notification.category,
      });
    } catch (error) {
      console.error('Failed to show notification:', error);
    }
  };

  // Get icon URL for category
  const getCategoryIcon = (category: NotificationCategory): string => {
    const icons: Record<NotificationCategory, string> = {
      [NotificationCategory.Transaction]: '💱',
      [NotificationCategory.Alert]: '⚠️',
      [NotificationCategory.Message]: '💬',
      [NotificationCategory.Price]: '📊',
      [NotificationCategory.Validator]: '⚡',
      [NotificationCategory.System]: '⚙️',
    };
    return icons[category];
  };

  // Play notification sound
  const playNotificationSound = (category: NotificationCategory) => {
    try {
      const audio = new Audio(`/sounds/notification-${category}.mp3`);
      audio.volume = 0.5;
      audio.play().catch(error => console.error('Failed to play sound:', error));
    } catch (error) {
      console.error('Failed to play notification sound:', error);
    }
  };

  // Mark notification as read
  const markAsRead = (id: string) => {
    setState(prev => {
      const updatedNotifications = prev.notifications.map(n =>
        n.id === id ? { ...n, read: true } : n
      );
      return {
        ...prev,
        notifications: updatedNotifications,
        unread_count: Math.max(0, prev.unread_count - 1),
      };
    });
  };

  // Mark all notifications as read
  const markAllAsRead = () => {
    setState(prev => ({
      ...prev,
      notifications: prev.notifications.map(n => ({ ...n, read: true })),
      unread_count: 0,
    }));
  };

  // Clear notifications
  const clearNotifications = () => {
    setState(prev => ({
      ...prev,
      notifications: [],
      unread_count: 0,
    }));
  };

  // Toggle category
  const toggleCategory = (category: NotificationCategory) => {
    setState(prev => {
      const newSettings = { ...prev.settings };
      const newCategorySettings = new Map(newSettings.category_settings);
      const currentValue = newCategorySettings.get(category) || false;
      newCategorySettings.set(category, !currentValue);
      newSettings.category_settings = newCategorySettings;

      return {
        ...prev,
        settings: newSettings,
      };
    });
  };

  // Filter notifications by category
  const filteredNotifications = selectedCategory === 'all'
    ? state.notifications
    : state.notifications.filter(n => n.category === selectedCategory);

  return (
    <div className="system-notifications">
      <div className="notifications-header">
        <h3>Notifications</h3>
        <div className="notification-controls">
          <div className="badge">{state.unread_count}</div>
          <button
            className="btn-icon"
            onClick={() => setShowHistory(!showHistory)}
            title="Toggle history"
          >
            📜
          </button>
          <button
            className="btn-icon"
            onClick={() => setShowHistory(false)}
            title="Clear all"
          >
            ✕
          </button>
        </div>
      </div>

      {/* DND Mode Toggle */}
      <div className="dnd-section">
        <label className="dnd-toggle">
          <input
            type="checkbox"
            checked={state.settings.do_not_disturb}
            onChange={e =>
              setState(prev => ({
                ...prev,
                settings: { ...prev.settings, do_not_disturb: e.target.checked },
              }))
            }
          />
          🔇 Do Not Disturb
        </label>
        {state.settings.do_not_disturb && (
          <div className="dnd-times">
            <label>
              From:
              <input
                type="time"
                value={state.settings.dnd_start}
                onChange={e =>
                  setState(prev => ({
                    ...prev,
                    settings: { ...prev.settings, dnd_start: e.target.value },
                  }))
                }
              />
            </label>
            <label>
              To:
              <input
                type="time"
                value={state.settings.dnd_end}
                onChange={e =>
                  setState(prev => ({
                    ...prev,
                    settings: { ...prev.settings, dnd_end: e.target.value },
                  }))
                }
              />
            </label>
          </div>
        )}
      </div>

      {/* Notification Settings */}
      <div className="notification-settings">
        <h4>Enable Categories</h4>
        <div className="category-toggles">
          {Array.from(state.settings.category_settings.entries()).map(([category, enabled]) => (
            <label key={category} className="category-toggle">
              <input
                type="checkbox"
                checked={enabled}
                onChange={() => toggleCategory(category)}
              />
              <span className="category-label">
                {getCategoryIcon(category as NotificationCategory)} {category}
              </span>
            </label>
          ))}
        </div>
      </div>

      {/* Sound & Desktop Settings */}
      <div className="global-settings">
        <label>
          <input
            type="checkbox"
            checked={state.settings.sound_enabled}
            onChange={e =>
              setState(prev => ({
                ...prev,
                settings: { ...prev.settings, sound_enabled: e.target.checked },
              }))
            }
          />
          🔊 Sound enabled
        </label>
        <label>
          <input
            type="checkbox"
            checked={state.settings.desktop_enabled}
            onChange={e =>
              setState(prev => ({
                ...prev,
                settings: { ...prev.settings, desktop_enabled: e.target.checked },
              }))
            }
          />
          🖥️ Desktop notifications
        </label>
      </div>

      {!showHistory ? (
        <>
          {/* Active Notifications */}
          <div className="notifications-list">
            {filteredNotifications.length === 0 ? (
              <div className="empty-state">No notifications</div>
            ) : (
              filteredNotifications.map(notification => (
                <div
                  key={notification.id}
                  className={`notification-item ${notification.category} ${
                    notification.read ? 'read' : 'unread'
                  }`}
                  onClick={() => markAsRead(notification.id)}
                >
                  <div className="notification-icon">
                    {getCategoryIcon(notification.category as any)}
                  </div>
                  <div className="notification-content">
                    <div className="notification-title">{notification.title}</div>
                    <div className="notification-body">{notification.body}</div>
                    <div className="notification-time">
                      {notification.timestamp.toLocaleTimeString()}
                    </div>
                  </div>
                  {notification.action_url && (
                    <button
                      className="notification-action"
                      onClick={e => {
                        e.stopPropagation();
                        window.open(notification.action_url);
                      }}
                    >
                      {notification.action_label || 'View'}
                    </button>
                  )}
                </div>
              ))
            )}
          </div>

          {/* Batch Actions */}
          <div className="batch-actions">
            <button onClick={markAllAsRead} disabled={state.unread_count === 0}>
              Mark all as read
            </button>
            <button onClick={clearNotifications} disabled={state.notifications.length === 0}>
              Clear all
            </button>
          </div>
        </>
      ) : (
        <>
          {/* History View */}
          <div className="history-section">
            <h4>Notification History ({state.history.length})</h4>
            <div className="history-list">
              {state.history.slice(0, 50).map(notification => (
                <div key={notification.id} className="history-item">
                  <div className="history-icon">
                    {getCategoryIcon(notification.category as any)}
                  </div>
                  <div className="history-text">
                    <strong>{notification.title}</strong>
                    <p>{notification.body}</p>
                    <small>{notification.timestamp.toLocaleString()}</small>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </>
      )}

      <style>{`
        .system-notifications {
          background: white;
          border-radius: 8px;
          padding: 16px;
          max-width: 400px;
          max-height: 600px;
          overflow-y: auto;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }

        .notifications-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 16px;
          padding-bottom: 12px;
          border-bottom: 1px solid #eee;
        }

        .notifications-header h3 {
          margin: 0;
          font-size: 18px;
        }

        .notification-controls {
          display: flex;
          gap: 8px;
          align-items: center;
        }

        .badge {
          background: #ff3b30;
          color: white;
          border-radius: 12px;
          width: 24px;
          height: 24px;
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 12px;
          font-weight: bold;
        }

        .btn-icon {
          background: none;
          border: none;
          cursor: pointer;
          font-size: 18px;
          padding: 4px;
          border-radius: 4px;
          transition: background 0.2s;
        }

        .btn-icon:hover {
          background: #f0f0f0;
        }

        .dnd-section {
          background: #f9f9f9;
          padding: 12px;
          border-radius: 6px;
          margin-bottom: 12px;
        }

        .dnd-toggle {
          display: flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
          font-weight: 500;
        }

        .dnd-times {
          display: flex;
          gap: 12px;
          margin-top: 8px;
          padding-top: 8px;
          border-top: 1px solid #eee;
        }

        .dnd-times label {
          display: flex;
          align-items: center;
          gap: 6px;
          font-size: 12px;
        }

        .dnd-times input {
          padding: 4px 8px;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 12px;
        }

        .notification-settings {
          background: #f9f9f9;
          padding: 12px;
          border-radius: 6px;
          margin-bottom: 12px;
        }

        .notification-settings h4 {
          margin: 0 0 8px 0;
          font-size: 12px;
          text-transform: uppercase;
          color: #666;
        }

        .category-toggles {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 8px;
        }

        .category-toggle {
          display: flex;
          align-items: center;
          gap: 6px;
          cursor: pointer;
          font-size: 12px;
        }

        .category-toggle input {
          cursor: pointer;
        }

        .global-settings {
          display: flex;
          flex-direction: column;
          gap: 8px;
          margin-bottom: 12px;
          padding: 12px;
          background: #f9f9f9;
          border-radius: 6px;
        }

        .global-settings label {
          display: flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
          font-size: 13px;
        }

        .global-settings input {
          cursor: pointer;
        }

        .notifications-list {
          margin-bottom: 12px;
        }

        .notification-item {
          display: flex;
          gap: 12px;
          padding: 12px;
          border: 1px solid #eee;
          border-radius: 6px;
          margin-bottom: 8px;
          cursor: pointer;
          transition: all 0.2s;
          background: white;
        }

        .notification-item:hover {
          background: #f9f9f9;
          border-color: #ddd;
        }

        .notification-item.unread {
          background: #f0f7ff;
          border-color: #0066cc;
        }

        .notification-icon {
          font-size: 20px;
          flex-shrink: 0;
        }

        .notification-content {
          flex: 1;
          overflow: hidden;
        }

        .notification-title {
          font-weight: 600;
          font-size: 13px;
          margin-bottom: 4px;
        }

        .notification-body {
          font-size: 12px;
          color: #666;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .notification-time {
          font-size: 11px;
          color: #999;
          margin-top: 4px;
        }

        .notification-action {
          padding: 4px 12px;
          background: #0066cc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 11px;
          white-space: nowrap;
        }

        .notification-action:hover {
          background: #0052a3;
        }

        .empty-state {
          text-align: center;
          color: #999;
          padding: 24px 12px;
          font-size: 13px;
        }

        .batch-actions {
          display: flex;
          gap: 8px;
          margin-top: 12px;
        }

        .batch-actions button {
          flex: 1;
          padding: 8px;
          background: #f0f0f0;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 12px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .batch-actions button:hover:not(:disabled) {
          background: #e0e0e0;
        }

        .batch-actions button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .history-section h4 {
          margin: 0 0 12px 0;
          font-size: 13px;
        }

        .history-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
          max-height: 350px;
          overflow-y: auto;
        }

        .history-item {
          display: flex;
          gap: 10px;
          padding: 10px;
          background: #f9f9f9;
          border-radius: 6px;
          border-left: 3px solid #0066cc;
        }

        .history-icon {
          font-size: 16px;
          flex-shrink: 0;
        }

        .history-text {
          flex: 1;
          overflow: hidden;
        }

        .history-text strong {
          display: block;
          font-size: 12px;
          margin-bottom: 2px;
        }

        .history-text p {
          margin: 2px 0;
          font-size: 11px;
          color: #666;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .history-text small {
          font-size: 10px;
          color: #999;
          display: block;
          margin-top: 4px;
        }
      `}</style>
    </div>
  );
};

export default SystemNotifications;
