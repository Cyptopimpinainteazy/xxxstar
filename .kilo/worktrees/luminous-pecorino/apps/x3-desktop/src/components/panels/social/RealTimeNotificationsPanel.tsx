import React, { useState } from "react";
import { Bell, Check, MapPin, Trash2, Settings, Eye, ArrowRight } from "lucide-react";
import clsx from "clsx";

interface Notification {
  id: string;
  type: "like" | "comment" | "follow" | "mention" | "share" | "tip" | "burn";
  from: string;
  content: string;
  timestamp: string;
  read: boolean;
  avatar?: string;
}

interface NotificationSettings {
  enabled: boolean;
  sound: boolean;
  vibration: boolean;
  showPreview: boolean;
  groupByType: boolean;
}

interface NotificationQueue {
  total: number;
  unread: number;
  delivered: number;
  failed: number;
}

const MOCK_NOTIFICATIONS: Notification[] = [
  {
    id: "1",
    type: "like",
    from: "Alice",
    content: "liked your token announcement post",
    timestamp: "2 mins ago",
    read: false,
    avatar: "AL",
  },
  {
    id: "2",
    type: "comment",
    from: "Bob",
    content: "replied to your message: 'Great insight!'",
    timestamp: "5 mins ago",
    read: false,
    avatar: "BO",
  },
  {
    id: "3",
    type: "follow",
    from: "Carol",
    content: "started following you",
    timestamp: "12 mins ago",
    read: true,
    avatar: "CA",
  },
  {
    id: "4",
    type: "mention",
    from: "David",
    content: "mentioned you in a community post",
    timestamp: "1 hour ago",
    read: true,
    avatar: "DA",
  },
  {
    id: "5",
    type: "tip",
    from: "Eve",
    content: "sent you a 50 X3 tip",
    timestamp: "2 hours ago",
    read: true,
    avatar: "EV",
  },
  {
    id: "6",
    type: "share",
    from: "Frank",
    content: "shared your post in #development",
    timestamp: "3 hours ago",
    read: true,
    avatar: "FR",
  },
];

const MOCK_QUEUE: NotificationQueue = {
  total: 127,
  unread: 2,
  delivered: 122,
  failed: 3,
};

const TYPE_COLOR: Record<string, string> = {
  like: "text-pink-400",
  comment: "text-blue-400",
  follow: "text-purple-400",
  mention: "text-yellow-400",
  share: "text-cyan-400",
  tip: "text-green-400",
  burn: "text-red-400",
};

const TYPE_BG: Record<string, string> = {
  like: "bg-pink-600/10",
  comment: "bg-blue-600/10",
  follow: "bg-purple-600/10",
  mention: "bg-yellow-600/10",
  share: "bg-cyan-600/10",
  tip: "bg-green-600/10",
  burn: "bg-red-600/10",
};

export default function RealTimeNotificationsPanel() {
  const [notifications, setNotifications] = useState<Notification[]>(MOCK_NOTIFICATIONS);
  const [queue, setQueue] = useState<NotificationQueue>(MOCK_QUEUE);
  const [activeTab, setActiveTab] = useState<"notifications" | "queue" | "settings">("notifications");
  const [settings, setSettings] = useState<NotificationSettings>({
    enabled: true,
    sound: true,
    vibration: true,
    showPreview: true,
    groupByType: false,
  });
  const [selectedNotification, setSelectedNotification] = useState<Notification | null>(null);

  const handleMarkAsRead = (id: string) => {
    setNotifications(notifications.map((n) => (n.id === id ? { ...n, read: true } : n)));
  };

  const handleDeleteNotification = (id: string) => {
    setNotifications(notifications.filter((n) => n.id !== id));
  };

  const handleMarkAllAsRead = () => {
    setNotifications(notifications.map((n) => ({ ...n, read: true })));
  };

  const handleClearAll = () => {
    setNotifications([]);
  };

  const handleRetryFailed = () => {
    setQueue({ ...queue, failed: 0, delivered: queue.delivered + 3 });
  };

  const unreadCount = notifications.filter((n) => !n.read).length;
  const notificationsByType = settings.groupByType
    ? notifications.reduce<Record<string, Notification[]>>((acc, n) => {
        (acc[n.type] ??= []).push(n);
        return acc;
      }, {})
    : {};

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Bell size={20} className="text-cyan-400" /> Real-Time Notifications
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total</div>
            <div className="text-lg font-bold text-cyan-400">{queue.total}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Unread</div>
            <div className={clsx("text-lg font-bold", unreadCount > 0 ? "text-red-400" : "text-green-400")}>
              {unreadCount}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Delivered</div>
            <div className="text-lg font-bold text-green-400">{queue.delivered}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Failed</div>
            <div className={clsx("text-lg font-bold", queue.failed > 0 ? "text-red-400" : "text-green-400")}>
              {queue.failed}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["notifications", "queue", "settings"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-cyan-600 text-cyan-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "queue" ? "Queue & Retry" : tab}
            </button>
          ))}
        </div>

        {activeTab === "notifications" && (
          <div className="space-y-2">
            {/* Toolbar */}
            <div className="flex gap-2 mb-3">
              <button
                onClick={handleMarkAllAsRead}
                className="bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 px-3 py-1.5 rounded text-xs font-semibold transition"
              >
                Mark All Read
              </button>
              <button
                onClick={handleClearAll}
                className="bg-[#15151b] border border-[#2a2a35] hover:border-red-600 px-3 py-1.5 rounded text-xs font-semibold transition"
              >
                Clear All
              </button>
            </div>

            {/* Notification List */}
            {notifications.length === 0 ? (
              <div className="text-center text-gray-500 py-8">No notifications</div>
            ) : settings.groupByType ? (
              Object.entries(notificationsByType).map(
                ([type, nots]) =>
                  nots && nots.length > 0 && (
                    <div key={type}>
                      <div className="text-xs font-semibold text-gray-400 uppercase mb-2">{type}</div>
                      <div className="space-y-2">
                        {nots.map((notif) => (
                          <NotificationItem
                            key={notif.id}
                            notification={notif}
                            onMarkRead={handleMarkAsRead}
                            onDelete={handleDeleteNotification}
                            onSelect={setSelectedNotification}
                          />
                        ))}
                      </div>
                    </div>
                  )
              )
            ) : (
              <div className="space-y-2">
                {notifications.map((notif) => (
                  <NotificationItem
                    key={notif.id}
                    notification={notif}
                    onMarkRead={handleMarkAsRead}
                    onDelete={handleDeleteNotification}
                    onSelect={setSelectedNotification}
                  />
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === "queue" && (
          <div className="space-y-3">
            {/* Queue Status */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm">WebSocket Queue Status</h3>

              <div className="space-y-2">
                <div className="flex justify-between items-center">
                  <span className="text-xs text-gray-400">Total in Queue</span>
                  <span className="font-bold">{queue.total}</span>
                </div>
                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-cyber to-cyan-600"
                    style={{ width: `${((queue.delivered / queue.total) * 100).toFixed(0)}%` }}
                  />
                </div>
              </div>

              <div className="grid grid-cols-3 gap-2 mt-4 text-xs">
                <div className="bg-[#2a2a35] rounded p-2">
                  <div className="text-gray-400 mb-1">Delivered</div>
                  <div className="font-bold text-green-400">{queue.delivered}</div>
                </div>
                <div className="bg-[#2a2a35] rounded p-2">
                  <div className="text-gray-400 mb-1">Pending</div>
                  <div className="font-bold text-yellow-400">{queue.total - queue.delivered - queue.failed}</div>
                </div>
                <div className="bg-[#2a2a35] rounded p-2">
                  <div className="text-gray-400 mb-1">Failed</div>
                  <div className="font-bold text-red-400">{queue.failed}</div>
                </div>
              </div>
            </div>

            {/* Retry Failed */}
            {queue.failed > 0 && (
              <div className="bg-red-600/10 border border-red-600/30 rounded-lg p-4">
                <div className="mb-2 font-semibold text-sm flex items-center gap-2">
                  <MapPin size={14} className="text-red-400" /> {queue.failed} Failed Notifications
                </div>
                <button
                  onClick={handleRetryFailed}
                  className="w-full bg-red-600/20 hover:bg-red-600/30 border border-red-600 rounded px-3 py-2 text-xs font-semibold text-red-400 transition"
                >
                  Retry Failed Deliveries
                </button>
              </div>
            )}

            {/* WebSocket Info */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-2 text-xs">
              <div className="flex justify-between">
                <span className="text-gray-400">Connection</span>
                <span className="flex items-center gap-1 text-green-400">
                  <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                  Connected
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Protocol</span>
                <span className="font-mono">WebSocket (wss://api.x3.local)</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Latency</span>
                <span className="font-mono">42ms avg</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Uptime</span>
                <span className="font-mono">99.98%</span>
              </div>
            </div>
          </div>
        )}

        {activeTab === "settings" && (
          <div className="space-y-3">
            <div className="space-y-2">
              {Object.entries(settings).map(([key, value]) => (
                <label key={key} className="flex items-center p-3 bg-[#15151b] border border-[#2a2a35] rounded-lg cursor-pointer hover:border-cyan-600 transition">
                  <input
                    type="checkbox"
                    checked={value}
                    onChange={(e) => setSettings({ ...settings, [key]: e.target.checked })}
                    className="w-4 h-4 accent-cyan-600 mr-3"
                  />
                  <div className="flex-1">
                    <div className="text-xs font-semibold capitalize">{key.replace(/([A-Z])/g, " $1").trim()}</div>
                    <div className="text-xs text-gray-500">
                      {key === "enabled" && "Enable all notifications"}
                      {key === "sound" && "Play sound on notification"}
                      {key === "vibration" && "Vibrate on notification"}
                      {key === "showPreview" && "Show notification preview"}
                      {key === "groupByType" && "Group notifications by type"}
                    </div>
                  </div>
                </label>
              ))}
            </div>

            {/* Notification Preferences */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 mt-4">
              <h3 className="text-xs font-semibold uppercase text-gray-400">Notification Preferences</h3>

              <div className="space-y-2 text-xs">
                {Object.entries(TYPE_COLOR).map(([type, color]) => (
                  <label key={type} className="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" defaultChecked className="w-3 h-3 accent-cyan-600" />
                    <span className={color + " font-semibold uppercase"}>{type}</span>
                  </label>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Real-time WebSocket push notifications with queue monitoring and delivery tracking.
      </div>
    </div>
  );
}

function NotificationItem({
  notification,
  onMarkRead,
  onDelete,
  onSelect,
}: {
  notification: Notification;
  onMarkRead: (id: string) => void;
  onDelete: (id: string) => void;
  onSelect: (n: Notification) => void;
}) {
  return (
    <div
      className={clsx(
        "p-3 rounded-lg border transition cursor-pointer hover:border-cyan-600",
        notification.read
          ? "bg-[#15151b] border-[#2a2a35]"
          : "bg-cyan-600/5 border-cyan-600/30"
      )}
      onClick={() => onSelect(notification)}
    >
      <div className="flex items-start gap-3">
        <div
          className={clsx(
            "w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 text-xs font-bold",
            TYPE_BG[notification.type],
            TYPE_COLOR[notification.type]
          )}
        >
          {notification.avatar}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <div>
              <div className="text-sm font-semibold">{notification.from}</div>
              <div className="text-xs text-gray-400">{notification.content}</div>
            </div>
            <div className="flex-shrink-0 text-xs text-gray-500">{notification.timestamp}</div>
          </div>
        </div>

        {!notification.read && (
          <div className="w-2 h-2 bg-cyan-400 rounded-full flex-shrink-0 mt-1" />
        )}
      </div>

      <div className="flex gap-2 mt-2 ml-11">
        {!notification.read && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onMarkRead(notification.id);
            }}
            className="text-xs text-gray-400 hover:text-cyan-400 transition flex items-center gap-1"
          >
            <Check size={12} /> Mark Read
          </button>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete(notification.id);
          }}
          className="text-xs text-gray-400 hover:text-red-400 transition flex items-center gap-1"
        >
          <Trash2 size={12} /> Delete
        </button>
      </div>
    </div>
  );
}
