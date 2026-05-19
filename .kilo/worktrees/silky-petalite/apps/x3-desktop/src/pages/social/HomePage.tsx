import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const HomePage: React.FC = () => {
  const {
    currentUser, stats, feed, friends, bulletins,
    loadFeed, loadFriends, loadBulletins, postStatus
  } = useSocialStore();
  const [statusText, setStatusText] = useState("");

  useEffect(() => {
    loadFeed();
    loadFriends();
    loadBulletins();
  }, [loadFeed, loadFriends, loadBulletins]);

  const handlePostStatus = async () => {
    if (!statusText.trim()) return;
    await postStatus(statusText.trim());
    setStatusText("");
  };

  const topFriends = friends.filter(f => f.isTopFriend).slice(0, 8);

  return (
    <div style={{ display: "grid", gridTemplateColumns: "280px 1fr", gap: "1.5rem" }}>
      {/* Left Sidebar */}
      <div>
        {/* User Card */}
        <div className="social-card">
          <div style={{ textAlign: "center" }}>
            {currentUser?.avatarUrl ? (
              <img src={currentUser.avatarUrl} alt="" style={{ width: 120, height: 120, borderRadius: 8, objectFit: "cover", border: "2px solid #ff6b35" }} />
            ) : (
              <div className="profile-avatar-placeholder" style={{ width: 120, height: 120, fontSize: "2.5rem", margin: "0 auto" }}>
                {currentUser?.displayName?.[0]?.toUpperCase() ?? "?"}
              </div>
            )}
            <div className="profile-name" style={{ fontSize: "1.1rem", marginTop: "0.5rem" }}>
              {currentUser?.displayName}
            </div>
            <div style={{ color: "#ff8c42", fontSize: "0.8rem", fontStyle: "italic" }}>
              {currentUser?.headline || "Edit your headline!"}
            </div>
            <div style={{ fontSize: "0.75rem", color: "#888", marginTop: "0.25rem" }}>
              Mood: {currentUser?.mood ?? "😊"}
            </div>
          </div>
          <div style={{ marginTop: "0.75rem" }}>
            <Link to="/social/profile/edit" className="social-btn social-btn-sm social-btn-outline" style={{ display: "block", textAlign: "center", textDecoration: "none" }}>
              Edit Profile
            </Link>
          </div>
        </div>

        {/* Quick Stats */}
        <div className="social-card">
          <div className="social-card-header">My Stats</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.5rem", fontSize: "0.75rem" }}>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.friendCount ?? 0}</span> friends</div>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.profileViews ?? 0}</span> views</div>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.photoCount ?? 0}</span> photos</div>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.musicCount ?? 0}</span> songs</div>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.blogCount ?? 0}</span> blogs</div>
            <div><span style={{ color: "#ff8c42", fontWeight: "bold" }}>{stats?.kudoCount ?? 0}</span> kudos</div>
          </div>
        </div>

        {/* Top Friends */}
        {topFriends.length > 0 && (
          <div className="social-card">
            <div className="social-card-header">My Top Friends</div>
            <div className="top-friends-grid">
              {topFriends.map(f => (
                <Link key={f.userId} to={`/social/view/${f.userId}`} className="friend-card-mini" style={{ textDecoration: "none" }}>
                  {f.avatarUrl ? (
                    <img src={f.avatarUrl} alt={f.displayName} />
                  ) : (
                    <div style={{ width: 60, height: 60, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "1.5rem", border: "2px solid #334" }}>
                      {f.displayName[0]?.toUpperCase()}
                    </div>
                  )}
                  <div className="friend-mini-name">{f.displayName}</div>
                </Link>
              ))}
            </div>
          </div>
        )}

        {/* Quick Links */}
        <div className="social-card">
          <div className="social-card-header">Quick Links</div>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
            <Link to="/social/messages" style={{ color: "#ccc", textDecoration: "none", fontSize: "0.8rem" }}>
              📬 Messages {(stats?.unreadMessages ?? 0) > 0 && <span className="nav-badge">{stats!.unreadMessages} new</span>}
            </Link>
            <Link to="/social/friends" style={{ color: "#ccc", textDecoration: "none", fontSize: "0.8rem" }}>
              👥 Friend Requests {(stats?.pendingRequests ?? 0) > 0 && <span className="nav-badge">{stats!.pendingRequests}</span>}
            </Link>
            <Link to="/social/bulletins" style={{ color: "#ccc", textDecoration: "none", fontSize: "0.8rem" }}>
              📋 Bulletins
            </Link>
            <Link to="/social/blog" style={{ color: "#ccc", textDecoration: "none", fontSize: "0.8rem" }}>
              📝 Blog
            </Link>
            <Link to="/social/groups" style={{ color: "#ccc", textDecoration: "none", fontSize: "0.8rem" }}>
              👥 Groups
            </Link>
          </div>
        </div>
      </div>

      {/* Main Feed */}
      <div>
        {/* Post Status */}
        <div className="social-card">
          <div className="social-card-header">What's on your mind?</div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <textarea
              className="social-textarea"
              style={{ minHeight: 60 }}
              placeholder="Share something with your friends..."
              value={statusText}
              onChange={e => setStatusText(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handlePostStatus(); } }}
            />
          </div>
          <div style={{ marginTop: "0.5rem", textAlign: "right" }}>
            <button className="social-btn social-btn-sm" onClick={handlePostStatus} disabled={!statusText.trim()}>
              Post Update
            </button>
          </div>
        </div>

        {/* Bulletins Preview */}
        {bulletins.length > 0 && (
          <div className="social-card">
            <div className="social-card-header">
              Latest Bulletins
              <Link to="/social/bulletins" style={{ float: "right", color: "#ff6b35", fontSize: "0.7rem", textDecoration: "none" }}>View All →</Link>
            </div>
            {bulletins.slice(0, 3).map(b => (
              <div key={b.id} className="bulletin-card">
                <div className="bulletin-title">{b.title}</div>
                <div className="bulletin-meta">
                  by <Link to={`/social/view/${b.userId}`} style={{ color: "#ff8c42", textDecoration: "none" }}>{b.displayName}</Link>
                  {" · "}{formatDistanceToNow(new Date(b.createdAt), { addSuffix: true })}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Feed */}
        <div className="social-card">
          <div className="social-card-header">Friend Activity</div>
          {feed.length === 0 ? (
            <div style={{ color: "#666", fontSize: "0.85rem", padding: "1rem", textAlign: "center" }}>
              No updates yet. Add some friends or post a status!
            </div>
          ) : (
            feed.map(s => (
              <div key={s.id} className="feed-post">
                {s.avatarUrl ? (
                  <img src={s.avatarUrl} alt="" />
                ) : (
                  <div style={{ width: 40, height: 40, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                    {s.displayName?.[0]?.toUpperCase()}
                  </div>
                )}
                <div>
                  <Link to={`/social/view/${s.userId}`} className="feed-author" style={{ textDecoration: "none" }}>
                    {s.displayName}
                  </Link>
                  <div className="feed-body">{s.body}</div>
                  <div className="feed-time">{formatDistanceToNow(new Date(s.createdAt), { addSuffix: true })}</div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default HomePage;
