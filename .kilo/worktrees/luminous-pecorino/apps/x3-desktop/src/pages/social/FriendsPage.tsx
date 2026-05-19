import React, { useEffect } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";


const FriendsPage: React.FC = () => {
  const { friends, pendingRequests, loadFriends, loadPendingRequests, respondFriendRequest, removeFriend, setTopFriends } = useSocialStore();

  useEffect(() => { loadFriends(); loadPendingRequests(); }, []);

  const topFriends = friends.filter(f => f.isTopFriend);

  const toggleTopFriend = async (friendUserId: string) => {
    const isTop = topFriends.some(f => f.userId === friendUserId);
    let newTopIds: string[];
    if (isTop) {
      newTopIds = topFriends.filter(f => f.userId !== friendUserId).map(f => f.userId);
    } else {
      if (topFriends.length >= 8) return; // max 8
      newTopIds = [...topFriends.map(f => f.userId), friendUserId];
    }
    await setTopFriends(newTopIds);
  };

  return (
    <div>
      <h1 className="social-page-title">Friends ({friends.length})</h1>

      {/* Pending Requests */}
      {pendingRequests.length > 0 && (
        <div className="social-card">
          <div className="social-card-header">Pending Friend Requests ({pendingRequests.length})</div>
          {pendingRequests.map(req => (
            <div key={req.id} style={{ display: "flex", alignItems: "center", gap: "0.75rem", padding: "0.5rem 0", borderBottom: "1px solid #223" }}>
              {req.fromAvatar ? (
                <img src={req.fromAvatar} alt="" style={{ width: 40, height: 40, borderRadius: 6, objectFit: "cover" }} />
              ) : (
                <div style={{ width: 40, height: 40, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center" }}>
                  {req.fromDisplayName[0]?.toUpperCase()}
                </div>
              )}
              <div style={{ flex: 1 }}>
                <Link to={`/social/view/${req.fromUserId}`} style={{ color: "#ff8c42", textDecoration: "none", fontWeight: "bold", fontSize: "0.85rem" }}>
                  {req.fromDisplayName}
                </Link>
                <div style={{ color: "#888", fontSize: "0.7rem" }}>@{req.fromUsername}</div>
              </div>
              <button className="social-btn social-btn-sm" onClick={() => respondFriendRequest(req.id, true)}>Accept</button>
              <button className="social-btn social-btn-sm social-btn-danger" onClick={() => respondFriendRequest(req.id, false)}>Deny</button>
            </div>
          ))}
        </div>
      )}

      {/* Top 8 */}
      {topFriends.length > 0 && (
        <div className="social-card">
          <div className="social-card-header">Top Friends</div>
          <div className="top-friends-grid" style={{ gridTemplateColumns: "repeat(4, 1fr)" }}>
            {topFriends.map(f => (
              <Link key={f.userId} to={`/social/view/${f.userId}`} className="friend-card-mini" style={{ textDecoration: "none" }}>
                {f.avatarUrl ? (
                  <img src={f.avatarUrl} alt={f.displayName} />
                ) : (
                  <div style={{ width: 60, height: 60, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "1.5rem", border: "2px solid #ff6b35", margin: "0 auto" }}>
                    {f.displayName[0]?.toUpperCase()}
                  </div>
                )}
                <div className="friend-mini-name">{f.displayName}</div>
                <span className={`profile-online-badge ${f.onlineStatus}`} style={{ fontSize: "0.6rem" }}>
                  {f.onlineStatus}
                </span>
              </Link>
            ))}
          </div>
        </div>
      )}

      {/* All Friends */}
      <div className="social-card">
        <div className="social-card-header">All Friends</div>
        {friends.length === 0 ? (
          <div style={{ color: "#666", fontSize: "0.85rem", padding: "1rem", textAlign: "center" }}>
            No friends yet. <Link to="/social/browse" style={{ color: "#ff6b35" }}>Browse</Link> to find people!
          </div>
        ) : (
          <div className="browse-grid" style={{ gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))" }}>
            {friends.map(f => (
              <div key={f.userId} className="browse-user-card" style={{ position: "relative" }}>
                <Link to={`/social/view/${f.userId}`} style={{ textDecoration: "none" }}>
                  {f.avatarUrl ? (
                    <img src={f.avatarUrl} alt={f.displayName} />
                  ) : (
                    <div style={{ width: 80, height: 80, borderRadius: 8, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "2rem", margin: "0 auto 0.5rem", border: "2px solid #334" }}>
                      {f.displayName[0]?.toUpperCase()}
                    </div>
                  )}
                  <div className="browse-user-name">{f.displayName}</div>
                  <div className="browse-user-headline">{f.headline || `@${f.username}`}</div>
                </Link>
                <div style={{ marginTop: "0.5rem", display: "flex", gap: "0.25rem", justifyContent: "center", flexWrap: "wrap" }}>
                  <button className="social-btn social-btn-sm social-btn-outline" style={{ fontSize: "0.6rem" }}
                    onClick={() => toggleTopFriend(f.userId)}>
                    {f.isTopFriend ? "★ Top" : "☆ Top"}
                  </button>
                  <button className="social-btn social-btn-sm social-btn-danger" style={{ fontSize: "0.6rem" }}
                    onClick={() => removeFriend(f.userId)}>
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default FriendsPage;
