import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const BlogPage: React.FC = () => {
  const { session, viewedBlogPosts, loadBlogPosts, createBlogPost, postBlogComment } = useSocialStore();
  const [composing, setComposing] = useState(false);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [mood, setMood] = useState("");
  const [commentTexts, setCommentTexts] = useState<Record<string, string>>({});

  useEffect(() => {
    if (session) loadBlogPosts(session.userId);
  }, [session?.userId]);

  const handlePost = async () => {
    if (!title.trim() || !body.trim()) return;
    await createBlogPost(title.trim(), body.trim(), mood || undefined);
    setTitle(""); setBody(""); setMood(""); setComposing(false);
    if (session) loadBlogPosts(session.userId);
  };

  const handleComment = async (blogPostId: string) => {
    const text = commentTexts[blogPostId]?.trim();
    if (!text) return;
    await postBlogComment(blogPostId, text);
    setCommentTexts(prev => ({ ...prev, [blogPostId]: "" }));
    if (session) loadBlogPosts(session.userId);
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1 className="social-page-title" style={{ margin: 0, border: "none", paddingBottom: 0 }}>My Blog</h1>
        <button className="social-btn social-btn-sm" onClick={() => setComposing(!composing)}>
          {composing ? "Cancel" : "New Blog Post"}
        </button>
      </div>

      {composing && (
        <div className="social-card" style={{ marginTop: "1rem" }}>
          <div className="social-card-header">Write a Blog Post</div>
          <div className="social-form-group">
            <label className="social-label">Title</label>
            <input className="social-input" value={title} onChange={e => setTitle(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Body</label>
            <textarea className="social-textarea" rows={8} value={body} onChange={e => setBody(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Mood (optional)</label>
            <input className="social-input" placeholder="How are you feeling?" value={mood} onChange={e => setMood(e.target.value)} />
          </div>
          <button className="social-btn" onClick={handlePost} disabled={!title.trim() || !body.trim()}>
            Publish
          </button>
        </div>
      )}

      <div style={{ marginTop: "1rem" }}>
        {viewedBlogPosts.length === 0 ? (
          <div className="social-card" style={{ color: "#666", textAlign: "center", padding: "2rem", fontSize: "0.85rem" }}>
            No blog posts yet. Write your first one!
          </div>
        ) : (
          viewedBlogPosts.map(bp => (
            <div key={bp.id} className="social-card">
              <div className="blog-post-title">{bp.title}</div>
              <div className="blog-post-meta">
                {formatDistanceToNow(new Date(bp.createdAt), { addSuffix: true })}
                {bp.mood && ` · Mood: ${bp.mood}`}
              </div>
              <div className="blog-post-body">{bp.body}</div>

              {/* Comments */}
              {bp.comments.length > 0 && (
                <div style={{ marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid #223" }}>
                  <div style={{ fontSize: "0.75rem", fontWeight: "bold", color: "#ff8c42", marginBottom: "0.5rem" }}>
                    Comments ({bp.comments.length})
                  </div>
                  {bp.comments.map(c => (
                    <div key={c.id} className="comment-card">
                      {c.authorAvatar ? (
                        <img src={c.authorAvatar} alt="" />
                      ) : (
                        <div style={{ width: 32, height: 32, borderRadius: 4, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, fontSize: "0.8rem" }}>
                          {c.authorDisplayName[0]?.toUpperCase()}
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
                  ))}
                </div>
              )}

              {/* Add comment */}
              <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.5rem" }}>
                <input
                  className="social-input"
                  placeholder="Leave a comment..."
                  value={commentTexts[bp.id] ?? ""}
                  onChange={e => setCommentTexts(prev => ({ ...prev, [bp.id]: e.target.value }))}
                  onKeyDown={e => { if (e.key === "Enter") handleComment(bp.id); }}
                />
                <button className="social-btn social-btn-sm" onClick={() => handleComment(bp.id)}>Post</button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default BlogPage;
