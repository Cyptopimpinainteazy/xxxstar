import React, { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";

const BrowsePage: React.FC = () => {
  const { browseResults, browse } = useSocialStore();
  const navigate = useNavigate();

  useEffect(() => { browse(); }, []);

  return (
    <div>
      <h1 className="social-page-title">Browse People</h1>

      <div className="social-card">
        {browseResults.length === 0 ? (
          <div style={{ color: "#666", textAlign: "center", padding: "2rem", fontSize: "0.85rem" }}>
            No users found. Be the first to join!
          </div>
        ) : (
          <div className="browse-grid">
            {browseResults.map(u => (
              <div key={u.id} className="browse-user-card" onClick={() => navigate(`/social/view/${u.id}`)}>
                {u.avatarUrl ? (
                  <img src={u.avatarUrl} alt={u.displayName} />
                ) : (
                  <div style={{ width: 80, height: 80, borderRadius: 8, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "2rem", margin: "0 auto 0.5rem", border: "2px solid #334" }}>
                    {u.displayName[0]?.toUpperCase()}
                  </div>
                )}
                <div className="browse-user-name">{u.displayName}</div>
                <div className="browse-user-headline">{u.headline || `@${u.username}`}</div>
                {u.location && <div className="browse-user-location">📍 {u.location}</div>}
                <span className={`profile-online-badge ${u.onlineStatus}`} style={{ fontSize: "0.6rem", marginTop: "0.25rem" }}>
                  {u.onlineStatus}
                </span>
              </div>
            ))}
          </div>
        )}

        {browseResults.length >= 40 && (
          <div style={{ textAlign: "center", marginTop: "1rem" }}>
            <button className="social-btn social-btn-outline" onClick={() => browse(browseResults.length)}>
              Load More
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default BrowsePage;
