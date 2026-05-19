import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const ProfilePage: React.FC = () => {
  const {
    session, currentUser, friends, music, photos, loadProfile,
    loadFriends, loadMusic, loadPhotos, loadComments, loadKudos,
    viewedComments, viewedKudos, loadBlogPosts, viewedBlogPosts,
    postComment, stats, loadStats
  } = useSocialStore();

  const [commentText, setCommentText] = useState("");

  useEffect(() => {
    loadProfile();
    loadFriends();
    loadMusic();
    loadPhotos();
    loadStats();
    if (session) {
      loadComments(session.userId);
      loadKudos(session.userId);
      loadBlogPosts(session.userId);
    }
  }, [session?.userId]);

  if (!currentUser) return <div style={{ color: "#888", textAlign: "center", padding: "2rem" }}>Loading...</div>;

  const topFriends = friends.filter(f => f.isTopFriend).slice(0, 8);
  const profileSong = music.find(m => m.isProfileSong);
  const [audioPlaying, setAudioPlaying] = useState(false);

  const handlePostComment = async () => {
    if (!commentText.trim() || !session) return;
    await postComment(session.userId, commentText.trim());
    setCommentText("");
    loadComments(session.userId);
  };

  return (
    <>
      {/* Custom CSS injection */}
      {currentUser.profileCss && <style>{currentUser.profileCss}</style>}

      <div className="profile-custom-css">
        {/* Header bar */}
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
          <h1 className="social-page-title" style={{ margin: 0, border: "none", paddingBottom: 0 }}>
            {currentUser.displayName}'s Profile
          </h1>
          <Link to="/social/profile/edit" className="social-btn social-btn-sm" style={{ textDecoration: "none" }}>
            Edit Profile
          </Link>
        </div>

        <div className="profile-container">
          {/* ─── Left Column ─── */}
          <div className="profile-left">
            {/* Avatar */}
            <div className="social-card">
              <div className="profile-avatar-box">
                {currentUser.avatarUrl ? (
                  <img src={currentUser.avatarUrl} alt={currentUser.displayName} />
                ) : (
                  <div className="profile-avatar-placeholder">
                    {currentUser.displayName[0]?.toUpperCase() ?? "?"}
                  </div>
                )}
                <div className="profile-name">
                  {currentUser.displayName}
                  {currentUser.role && currentUser.role !== "user" && (
                    <span className={`role-badge role-badge-${currentUser.role}`} style={{ marginLeft: 6 }}>
                      {currentUser.role === "team" ? "🔶 Team" : currentUser.role === "admin" ? "👑 Admin" : currentUser.role === "vip" ? "💎 VIP" : currentUser.role}
                    </span>
                  )}
                </div>
                <div className="profile-headline">{currentUser.headline || "No headline yet"}</div>
                <div className="profile-mood">Mood: {currentUser.mood}</div>
                <div className="mt-2">
                  <span className={`profile-online-badge ${currentUser.onlineStatus}`}>
                    {currentUser.onlineStatus === "online" ? "🟢 Online" : "⚫ Offline"}
                  </span>
                </div>
                <div className="profile-stats-row mt-2" style={{ justifyContent: "center" }}>
                  <span>{stats?.profileViews ?? currentUser.profileViews} views</span>
                  <span>{stats?.friendCount ?? 0} friends</span>
                </div>
              </div>
            </div>

            {/* URL / Contact */}
            <div className="social-card">
              <div className="social-card-header">Contacting {currentUser.displayName}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
                <Link to="/social/messages" className="social-btn social-btn-sm social-btn-outline" style={{ textDecoration: "none", textAlign: "center" }}>
                  Send Message
                </Link>
                <Link to="/social/friends" className="social-btn social-btn-sm social-btn-outline" style={{ textDecoration: "none", textAlign: "center" }}>
                  View Friends
                </Link>
              </div>
            </div>

            {/* Profile Song */}
            {profileSong && (
              <div className="social-card">
                <div className="social-card-header">🎵 Profile Song</div>
                <div className="music-player-bar">
                  <button className="play-btn" onClick={() => setAudioPlaying(!audioPlaying)}>
                    {audioPlaying ? "⏸" : "▶"}
                  </button>
                  <div className="music-track-info">
                    <div className="music-track-title">{profileSong.title}</div>
                    <div className="music-track-artist">{profileSong.artist || "Unknown"}</div>
                  </div>
                </div>
                {audioPlaying && profileSong.filePath && (
                  <audio src={profileSong.filePath} autoPlay onEnded={() => setAudioPlaying(false)} />
                )}
              </div>
            )}

            {/* Kudos */}
            {viewedKudos.length > 0 && (
              <div className="social-card">
                <div className="social-card-header">Kudos Received</div>
                <div className="kudo-grid">
                  {viewedKudos.map(k => (
                    <div key={k.id} className="kudo-badge">
                      {k.kind === "cool" ? "😎" : k.kind === "sweet" ? "🍬" : k.kind === "hot" ? "🔥" : "⭐"}
                      {k.kind} from {k.fromUsername}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Photos preview */}
            {photos.length > 0 && (
              <div className="social-card">
                <div className="social-card-header">
                  Photos
                  <Link to="/social/photos" style={{ float: "right", color: "#ff6b35", fontSize: "0.7rem", textDecoration: "none" }}>
                    View All ({photos.length}) →
                  </Link>
                </div>
                <div className="photo-grid" style={{ gridTemplateColumns: "repeat(3, 1fr)" }}>
                  {photos.slice(0, 6).map(p => (
                    <img key={p.id} src={p.filePath} alt={p.caption} />
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* ─── Right Column ─── */}
          <div className="profile-right">
            {/* Bio Details */}
            <div className="social-card">
              <div className="social-card-header">{currentUser.displayName}'s Details</div>
              <table className="profile-details-table">
                <tbody>
                  {currentUser.gender && <tr><td>Gender:</td><td>{currentUser.gender}</td></tr>}
                  {currentUser.age > 0 && <tr><td>Age:</td><td>{currentUser.age}</td></tr>}
                  {currentUser.location && <tr><td>Location:</td><td>{currentUser.location}</td></tr>}
                  {currentUser.status && <tr><td>Status:</td><td>{currentUser.status}</td></tr>}
                  {currentUser.zodiacSign && <tr><td>Zodiac:</td><td>{currentUser.zodiacSign}</td></tr>}
                  {currentUser.occupation && <tr><td>Occupation:</td><td>{currentUser.occupation}</td></tr>}
                  {currentUser.education && <tr><td>Education:</td><td>{currentUser.education}</td></tr>}
                  <tr><td>Member since:</td><td>{new Date(currentUser.createdAt).toLocaleDateString()}</td></tr>
                </tbody>
              </table>
            </div>

            {/* About Me */}
            {currentUser.aboutMe && (
              <div className="social-card">
                <div className="social-card-header">About Me</div>
                <div className="profile-section-body">{currentUser.aboutMe}</div>
              </div>
            )}

            {/* Who I'd Like to Meet */}
            {currentUser.whoIdLikeToMeet && (
              <div className="social-card">
                <div className="social-card-header">Who I'd Like to Meet</div>
                <div className="profile-section-body">{currentUser.whoIdLikeToMeet}</div>
              </div>
            )}

            {/* Interests */}
            {(currentUser.interests || currentUser.musicInterests || currentUser.movieInterests) && (
              <div className="social-card">
                <div className="social-card-header">{currentUser.displayName}'s Interests</div>
                <table className="profile-details-table">
                  <tbody>
                    {currentUser.interests && <tr><td>General:</td><td>{currentUser.interests}</td></tr>}
                    {currentUser.musicInterests && <tr><td>Music:</td><td>{currentUser.musicInterests}</td></tr>}
                    {currentUser.movieInterests && <tr><td>Movies:</td><td>{currentUser.movieInterests}</td></tr>}
                  </tbody>
                </table>
              </div>
            )}

            {/* Top Friends */}
            {topFriends.length > 0 && (
              <div className="social-card">
                <div className="social-card-header">
                  {currentUser.displayName}'s Top Friends
                  <Link to="/social/friends" style={{ float: "right", color: "#ff6b35", fontSize: "0.7rem", textDecoration: "none" }}>
                    View All →
                  </Link>
                </div>
                <div className="top-friends-grid">
                  {topFriends.map(f => (
                    <Link key={f.userId} to={`/social/view/${f.userId}`} className="friend-card-mini" style={{ textDecoration: "none" }}>
                      {f.avatarUrl ? (
                        <img src={f.avatarUrl} alt={f.displayName} />
                      ) : (
                        <div style={{ width: 60, height: 60, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "1.5rem", border: "2px solid #334", margin: "0 auto" }}>
                          {f.displayName[0]?.toUpperCase()}
                        </div>
                      )}
                      <div className="friend-mini-name">{f.displayName}</div>
                    </Link>
                  ))}
                </div>
              </div>
            )}

            {/* Blog posts preview */}
            {viewedBlogPosts.length > 0 && (
              <div className="social-card">
                <div className="social-card-header">
                  Latest Blog Posts
                  <Link to="/social/blog" style={{ float: "right", color: "#ff6b35", fontSize: "0.7rem", textDecoration: "none" }}>View All →</Link>
                </div>
                {viewedBlogPosts.slice(0, 3).map(bp => (
                  <div key={bp.id} className="blog-post-card">
                    <div className="blog-post-title">{bp.title}</div>
                    <div className="blog-post-meta">
                      {formatDistanceToNow(new Date(bp.createdAt), { addSuffix: true })}
                      {bp.mood && ` · Mood: ${bp.mood}`}
                    </div>
                    <div className="blog-post-body" style={{ maxHeight: 100, overflow: "hidden" }}>
                      {bp.body}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Comments */}
            <div className="social-card">
              <div className="social-card-header">
                {currentUser.displayName}'s Comments ({viewedComments.length})
              </div>
              {/* Post a comment */}
              <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
                <input
                  className="social-input"
                  placeholder="Leave a comment..."
                  value={commentText}
                  onChange={e => setCommentText(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter") handlePostComment(); }}
                />
                <button className="social-btn social-btn-sm" onClick={handlePostComment}>Post</button>
              </div>
              {viewedComments.length === 0 ? (
                <div style={{ color: "#666", fontSize: "0.8rem" }}>No comments yet.</div>
              ) : (
                viewedComments.map(c => (
                  <div key={c.id} className="comment-card">
                    {c.authorAvatar ? (
                      <img src={c.authorAvatar} alt="" />
                    ) : (
                      <div style={{ width: 40, height: 40, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                        {c.authorDisplayName?.[0]?.toUpperCase()}
                      </div>
                    )}
                    <div className="comment-body">
                      <Link to={`/social/view/${c.authorUserId}`} className="comment-author" style={{ textDecoration: "none" }}>
                        {c.authorDisplayName}
                      </Link>
                      <div className="comment-text">{c.body}</div>
                      <div className="comment-time">{formatDistanceToNow(new Date(c.createdAt), { addSuffix: true })}</div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export default ProfilePage;
