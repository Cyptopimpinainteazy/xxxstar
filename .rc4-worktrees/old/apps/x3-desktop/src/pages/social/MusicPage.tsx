import React, { useEffect, useState, useRef } from "react";
import { useSocialStore } from "@/stores/socialStore";

const MusicPage: React.FC = () => {
  const { music, loadMusic, addMusic, setProfileSong } = useSocialStore();
  const [title, setTitle] = useState("");
  const [artist, setArtist] = useState("");
  const [filePath, setFilePath] = useState("");
  const [playingId, setPlayingId] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  useEffect(() => { loadMusic(); }, []);

  const handleAdd = async () => {
    if (!title.trim() || !filePath.trim()) return;
    await addMusic(title.trim(), artist.trim(), filePath.trim());
    setTitle(""); setArtist(""); setFilePath("");
  };

  const togglePlay = (trackId: string, path: string) => {
    if (playingId === trackId) {
      audioRef.current?.pause();
      setPlayingId(null);
    } else {
      if (audioRef.current) audioRef.current.pause();
      const audio = new Audio(path);
      audioRef.current = audio;
      audio.play();
      audio.onended = () => setPlayingId(null);
      setPlayingId(trackId);
    }
  };

  return (
    <div>
      <h1 className="social-page-title">My Music ({music.length})</h1>

      {/* Add Music */}
      <div className="social-card">
        <div className="social-card-header">Add a Track</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.5rem" }}>
          <div className="social-form-group">
            <label className="social-label">Title</label>
            <input className="social-input" value={title} onChange={e => setTitle(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Artist</label>
            <input className="social-input" value={artist} onChange={e => setArtist(e.target.value)} />
          </div>
        </div>
        <div className="social-form-group">
          <label className="social-label">File URL / Path</label>
          <input className="social-input" placeholder="https://... or local path" value={filePath} onChange={e => setFilePath(e.target.value)} />
        </div>
        <button className="social-btn social-btn-sm" onClick={handleAdd} disabled={!title.trim() || !filePath.trim()}>
          Add Track
        </button>
      </div>

      {/* Music List */}
      <div className="social-card">
        {music.length === 0 ? (
          <div style={{ color: "#666", textAlign: "center", padding: "2rem", fontSize: "0.85rem" }}>
            No music yet. Add some tracks above!
          </div>
        ) : (
          music.map(t => (
            <div key={t.id} className="music-player-bar" style={{ marginBottom: "0.5rem" }}>
              <button className="play-btn" onClick={() => togglePlay(t.id, t.filePath)}>
                {playingId === t.id ? "⏸" : "▶"}
              </button>
              <div className="music-track-info" style={{ flex: 1 }}>
                <div className="music-track-title">
                  {t.title}
                  {t.isProfileSong && <span style={{ color: "#ff6b35", fontSize: "0.7rem", marginLeft: "0.5rem" }}>★ Profile Song</span>}
                </div>
                <div className="music-track-artist">{t.artist || "Unknown Artist"}</div>
              </div>
              <button
                className={`social-btn social-btn-sm ${t.isProfileSong ? "social-btn-outline" : ""}`}
                onClick={() => setProfileSong(t.id)}
              >
                {t.isProfileSong ? "★ Profile Song" : "Set as Profile Song"}
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default MusicPage;
