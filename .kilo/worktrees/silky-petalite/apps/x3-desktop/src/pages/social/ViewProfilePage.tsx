import React, { useEffect, useRef } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const fmt = (iso: string) => {
  try { return formatDistanceToNow(new Date(iso), { addSuffix: true }); } catch { return iso; }
};

const ViewProfilePage: React.FC = () => {
  const { userId } = useParams<{ userId: string }>();
  const navigate = useNavigate();
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const {
    session, viewedProfile, viewedFriends, viewedComments, viewedPhotos,
    viewedMusic, viewedBlogPosts, viewedKudos,
    viewProfile, sendFriendRequest, sendKudo, postComment, loading,
  } = useSocialStore();

  useEffect(() => {
    if (userId) viewProfile(userId);
  }, [userId]);

  const [commentBody, setCommentBody] = React.useState("");

  const handleComment = async () => {
    if (!commentBody.trim() || !userId) return;
    await postComment(userId, commentBody.trim());
    setCommentBody("");
    viewProfile(userId);
  };

  if (!viewedProfile) {
    return <div className="social-card" style={{ textAlign: "center", padding: "3rem" }}>
      {loading ? "Loading profile..." : "User not found."}
    </div>;
  }

  const u = viewedProfile;
  const isOwnProfile = session?.userId === u.id;
  const profileSong = viewedMusic.find(m => m.isProfileSong);

  return (
    <div>
      {u.profileCss && <style>{u.profileCss}</style>}

      <div className="profile-container">
        {/* Left column */}
        <div className="profile-left">
          <div className="social-card">
            <img className="profile-avatar"
              src={u.avatarUrl || `https://api.dicebear.com/7.x/pixel-art/svg?seed=${u.username}&size=200`}
              alt={u.displayName} />
            <h2 style={{ margin: "0.5rem 0 0.25rem" }}>
              {u.displayName}
              {u.role && u.role !== "user" && (
                <span className={`role-badge role-badge-${u.role}`} style={{ marginLeft: 8, verticalAlign: "middle" }}>
                  {u.role === "team" ? "🔶 Team" : u.role === "admin" ? "👑 Admin" : u.role === "vip" ? "💎 VIP" : u.role}
                </span>
              )}
            </h2>
            <div style={{ color: "#999", fontSize: "0.8rem" }}>@{u.username}</div>
            {u.headline && <div style={{ marginTop: "0.5rem", fontStyle: "italic", fontSize: "0.85rem" }}>{u.headline}</div>}
            <div style={{ display: "flex", gap: "0.5rem", marginTop: "1rem", flexWrap: "wrap" }}>
              {!isOwnProfile && session && (
                <>
                  <button className="social-btn social-btn-sm" onClick={() => sendFriendRequest(u.id)}>
                    Add Friend
                  </button>
                  <button className="social-btn social-btn-sm social-btn-outline"
                    onClick={() => navigate("/social/messages")}>
                    Send Message
                  </button>
                  <button className="social-btn social-btn-sm social-btn-outline"
                    onClick={() => sendKudo(u.id, "props")}>
                    Give Props 👊
                  </button>
                </>
              )}
              {isOwnProfile && (
                <button className="social-btn social-btn-sm" onClick={() => navigate("/social/profile/edit")}>
                  Edit Profile
                </button>
              )}
            </div>
          </div>

          {/* Profile song */}
          {profileSong && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">♫ Profile Song</div>
              <div style={{ fontSize: "0.85rem", marginBottom: "0.5rem" }}>
                {profileSong.title} — {profileSong.artist}
              </div>
              <audio ref={audioRef} src={profileSong.filePath} controls style={{ width: "100%" }} />
            </div>
          )}

          {/* Kudos */}
          {viewedKudos.length > 0 && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">Kudos ({viewedKudos.length})</div>
              <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
                {viewedKudos.map(k => (
                  <span key={k.id} className="kudo-badge">{k.kind === "props" ? "👊" : k.kind === "cool" ? "😎" : "⭐"}</span>
                ))}
              </div>
            </div>
          )}

          {/* Photos preview */}
          {viewedPhotos.length > 0 && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">Photos ({viewedPhotos.length})</div>
              <div className="photo-grid" style={{ gridTemplateColumns: "repeat(3, 1fr)" }}>
                {viewedPhotos.slice(0, 6).map(p => (
                  <img key={p.id} src={p.filePath} alt={p.caption || "photo"} style={{ width: "100%", borderRadius: "4px", aspectRatio: "1", objectFit: "cover" }} />
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Right column */}
        <div className="profile-right">
          {/* Details table */}
          <div className="social-card">
            <div className="social-card-header">{u.displayName}'s Details</div>
            <table className="profile-details-table">
              <tbody>
                {u.gender && <tr><td>Gender</td><td>{u.gender}</td></tr>}
                {u.age && <tr><td>Age</td><td>{u.age}</td></tr>}
                {u.location && <tr><td>Location</td><td>{u.location}</td></tr>}
                {u.zodiacSign && <tr><td>Zodiac Sign</td><td>{u.zodiacSign}</td></tr>}
                {u.occupation && <tr><td>Occupation</td><td>{u.occupation}</td></tr>}
                <tr><td>Member Since</td><td>{new Date(u.createdAt).toLocaleDateString()}</td></tr>
                <tr><td>Last Login</td><td>{fmt(u.lastLogin)}</td></tr>
              </tbody>
            </table>
          </div>

          {/* About me */}
          {u.aboutMe && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">About Me</div>
              <div style={{ whiteSpace: "pre-wrap", fontSize: "0.85rem", lineHeight: 1.6 }}>{u.aboutMe}</div>
            </div>
          )}

          {/* Who I'd like to meet */}
          {u.whoIdLikeToMeet && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">Who I'd Like to Meet</div>
              <div style={{ whiteSpace: "pre-wrap", fontSize: "0.85rem", lineHeight: 1.6 }}>{u.whoIdLikeToMeet}</div>
            </div>
          )}

          {/* Interests */}
          {u.interests && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">Interests</div>
              <div style={{ whiteSpace: "pre-wrap", fontSize: "0.85rem", lineHeight: 1.6 }}>{u.interests}</div>
            </div>
          )}

          {/* Top Friends */}
          {viewedFriends.length > 0 && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">{u.displayName}'s Friends ({viewedFriends.length})</div>
              <div className="top-friends-grid">
                {viewedFriends.slice(0, 8).map(f => (
                  <div key={f.userId} className="top-friend" onClick={() => navigate(`/social/view/${f.userId}`)} style={{ cursor: "pointer" }}>
                    <img src={f.avatarUrl || `https://api.dicebear.com/7.x/pixel-art/svg?seed=${f.username}`}
                      alt={f.displayName} style={{ width: 60, height: 60, borderRadius: "4px", objectFit: "cover" }} />
                    <div className="top-friend-name">{f.displayName}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Blog posts preview */}
          {viewedBlogPosts.length > 0 && (
            <div className="social-card" style={{ marginTop: "0.75rem" }}>
              <div className="social-card-header">{u.displayName}'s Blog</div>
              {viewedBlogPosts.slice(0, 3).map(bp => (
                <div key={bp.id} className="blog-post-card" style={{ marginBottom: "0.75rem" }}>
                  <h4 style={{ margin: "0 0 0.25rem" }}>{bp.title}</h4>
                  <div style={{ fontSize: "0.8rem", color: "#999" }}>{fmt(bp.createdAt)}</div>
                  <p style={{ fontSize: "0.85rem", lineHeight: 1.5, margin: "0.5rem 0 0" }}>
                    {bp.body.length > 200 ? bp.body.slice(0, 200) + "..." : bp.body}
                  </p>
                </div>
              ))}
            </div>
          )}

          {/* Comments */}
          <div className="social-card" style={{ marginTop: "0.75rem" }}>
            <div className="social-card-header">Comments ({viewedComments.length})</div>
            {session && (
              <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
                <textarea className="social-textarea" style={{ flex: 1 }} rows={2}
                  placeholder="Leave a comment..." value={commentBody}
                  onChange={e => setCommentBody(e.target.value)} />
                <button className="social-btn social-btn-sm" style={{ alignSelf: "flex-end" }}
                  onClick={handleComment} disabled={!commentBody.trim()}>Post</button>
              </div>
            )}
            {viewedComments.length === 0 ? (
              <div style={{ color: "#666", fontSize: "0.85rem" }}>No comments yet.</div>
            ) : (
              viewedComments.map(c => (
                <div key={c.id} className="comment-card">
                  <strong style={{ color: "#ff6b35", cursor: "pointer" }}
                    onClick={() => navigate(`/social/view/${c.authorUserId}`)}>
                    {c.authorDisplayName}
                  </strong>
                  <span style={{ color: "#666", fontSize: "0.75rem", marginLeft: "0.5rem" }}>{fmt(c.createdAt)}</span>
                  <p style={{ margin: "0.25rem 0 0", fontSize: "0.85rem" }}>{c.body}</p>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default ViewProfilePage;
