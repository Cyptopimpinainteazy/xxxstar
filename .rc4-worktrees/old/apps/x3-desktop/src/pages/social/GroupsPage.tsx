import React, { useEffect, useState } from "react";
import { useSocialStore } from "@/stores/socialStore";

const GroupsPage: React.FC = () => {
  const { groups, loadGroups, createGroup, joinGroup } = useSocialStore();
  const [composing, setComposing] = useState(false);
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [category, setCategory] = useState("general");

  useEffect(() => { loadGroups(); }, []);

  const handleCreate = async () => {
    if (!name.trim()) return;
    await createGroup(name.trim(), desc.trim(), category);
    setName(""); setDesc(""); setComposing(false);
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1 className="social-page-title" style={{ margin: 0, border: "none", paddingBottom: 0 }}>Groups</h1>
        <button className="social-btn social-btn-sm" onClick={() => setComposing(!composing)}>
          {composing ? "Cancel" : "Create Group"}
        </button>
      </div>

      {composing && (
        <div className="social-card" style={{ marginTop: "1rem" }}>
          <div className="social-card-header">Create a Group</div>
          <div className="social-form-group">
            <label className="social-label">Group Name</label>
            <input className="social-input" value={name} onChange={e => setName(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Description</label>
            <textarea className="social-textarea" rows={3} value={desc} onChange={e => setDesc(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Category</label>
            <select className="social-select" style={{ width: "100%" }} value={category} onChange={e => setCategory(e.target.value)}>
              <option value="general">General</option>
              <option value="music">Music</option>
              <option value="gaming">Gaming</option>
              <option value="art">Art</option>
              <option value="tech">Technology</option>
              <option value="sports">Sports</option>
              <option value="crypto">Crypto</option>
              <option value="memes">Memes</option>
            </select>
          </div>
          <button className="social-btn" onClick={handleCreate} disabled={!name.trim()}>Create Group</button>
        </div>
      )}

      <div className="social-card" style={{ marginTop: "1rem" }}>
        {groups.length === 0 ? (
          <div style={{ color: "#666", textAlign: "center", padding: "2rem", fontSize: "0.85rem" }}>
            No groups yet. Create one above!
          </div>
        ) : (
          <div className="group-grid">
            {groups.map(g => (
              <div key={g.id} className="group-card">
                <div className="group-name">{g.name}</div>
                <div className="group-desc">{g.description || "No description"}</div>
                <div className="group-meta">
                  📁 {g.category} · 👥 {g.memberCount} members
                </div>
                <button className="social-btn social-btn-sm social-btn-outline" style={{ marginTop: "0.5rem" }}
                  onClick={() => joinGroup(g.id)}>
                  Join Group
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default GroupsPage;
