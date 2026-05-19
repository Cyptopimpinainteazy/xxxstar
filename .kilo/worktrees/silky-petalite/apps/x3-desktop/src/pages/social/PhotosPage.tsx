import React, { useEffect, useState } from "react";
import { useSocialStore } from "@/stores/socialStore";

const PhotosPage: React.FC = () => {
  const { photos, loadPhotos, addPhoto, deletePhoto } = useSocialStore();
  const [url, setUrl] = useState("");
  const [caption, setCaption] = useState("");
  const [album, setAlbum] = useState("");
  const [lightbox, setLightbox] = useState<string | null>(null);

  useEffect(() => { loadPhotos(); }, []);

  const handleAdd = async () => {
    if (!url.trim()) return;
    await addPhoto(url.trim(), caption.trim(), album.trim() || undefined);
    setUrl(""); setCaption(""); setAlbum("");
  };

  return (
    <div>
      <h1 className="social-page-title">My Photos ({photos.length})</h1>

      {/* Upload */}
      <div className="social-card">
        <div className="social-card-header">Add a Photo</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.5rem" }}>
          <div className="social-form-group">
            <label className="social-label">Image URL</label>
            <input className="social-input" placeholder="https://..." value={url} onChange={e => setUrl(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Album (optional)</label>
            <input className="social-input" placeholder="Default" value={album} onChange={e => setAlbum(e.target.value)} />
          </div>
        </div>
        <div className="social-form-group">
          <label className="social-label">Caption</label>
          <input className="social-input" value={caption} onChange={e => setCaption(e.target.value)} />
        </div>
        <button className="social-btn social-btn-sm" onClick={handleAdd} disabled={!url.trim()}>Add Photo</button>
      </div>

      {/* Gallery */}
      <div className="social-card">
        {photos.length === 0 ? (
          <div style={{ color: "#666", textAlign: "center", padding: "2rem", fontSize: "0.85rem" }}>
            No photos yet. Add some above!
          </div>
        ) : (
          <div className="photo-grid">
            {photos.map(p => (
              <div key={p.id} style={{ position: "relative" }}>
                <img src={p.filePath} alt={p.caption} onClick={() => setLightbox(p.filePath)} />
                <button
                  onClick={() => deletePhoto(p.id)}
                  style={{ position: "absolute", top: 4, right: 4, background: "rgba(0,0,0,0.7)", color: "#ff4444", border: "none", borderRadius: 4, padding: "2px 6px", fontSize: "0.7rem", cursor: "pointer" }}
                >
                  ✕
                </button>
                {p.caption && (
                  <div style={{ fontSize: "0.7rem", color: "#999", marginTop: "0.25rem", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {p.caption}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Lightbox */}
      {lightbox && (
        <div
          onClick={() => setLightbox(null)}
          style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.9)", zIndex: 9999, display: "flex", alignItems: "center", justifyContent: "center", cursor: "pointer" }}
        >
          <img src={lightbox} alt="" style={{ maxWidth: "90vw", maxHeight: "90vh", borderRadius: 8 }} />
        </div>
      )}
    </div>
  );
};

export default PhotosPage;
