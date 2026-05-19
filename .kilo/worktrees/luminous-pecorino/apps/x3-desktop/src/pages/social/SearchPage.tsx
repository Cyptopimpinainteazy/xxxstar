import React, { useState } from "react";
import { useSocialStore } from "@/stores/socialStore";
import { useNavigate } from "react-router-dom";

const SearchPage: React.FC = () => {
  const { searchResults, search, sendFriendRequest, loading, session } = useSocialStore();
  const [query, setQuery] = useState("");
  const navigate = useNavigate();

  const handleSearch = async () => {
    if (!query.trim()) return;
    await search(query.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleSearch();
  };

  return (
    <div>
      <h1 className="social-page-title">Search Users</h1>

      <div className="social-card">
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <input className="social-input" style={{ flex: 1 }} value={query}
            onChange={e => setQuery(e.target.value)} onKeyDown={handleKeyDown}
            placeholder="Search by name, username, or location..." />
          <button className="social-btn" onClick={handleSearch} disabled={loading || !query.trim()}>
            {loading ? "Searching..." : "Search"}
          </button>
        </div>
      </div>

      {searchResults.length > 0 && (
        <div className="social-card" style={{ marginTop: "1rem" }}>
          <div className="social-card-header">Results ({searchResults.length})</div>
          <div className="browse-grid">
            {searchResults.map(u => (
              <div key={u.id} className="browse-card" onClick={() => navigate(`/social/view/${u.id}`)}
                style={{ cursor: "pointer" }}>
                <img className="browse-avatar"
                  src={u.avatarUrl || `https://api.dicebear.com/7.x/pixel-art/svg?seed=${u.username}`}
                  alt={u.displayName} />
                <div className="browse-name">{u.displayName}</div>
                <div className="browse-meta">@{u.username}</div>
                {u.location && <div className="browse-meta">{u.location}</div>}
                {u.headline && <div className="browse-meta" style={{ marginTop: "0.25rem", fontStyle: "italic" }}>{u.headline}</div>}
                {session && u.id !== session.userId && (
                  <button className="social-btn social-btn-sm social-btn-outline" style={{ marginTop: "0.5rem" }}
                    onClick={(e) => { e.stopPropagation(); sendFriendRequest(u.id); }}>
                    Add Friend
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {searchResults.length === 0 && query && !loading && (
        <div className="social-card" style={{ marginTop: "1rem", textAlign: "center", color: "#666", padding: "2rem", fontSize: "0.85rem" }}>
          No users found for "{query}". Try a different search.
        </div>
      )}
    </div>
  );
};

export default SearchPage;
