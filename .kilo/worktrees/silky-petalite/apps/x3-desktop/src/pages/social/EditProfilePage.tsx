import React, { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";

const MOODS = ["happy", "sad", "excited", "bored", "angry", "thoughtful", "creative", "sleepy", "hungry", "grateful", "loved", "anxious", "chill", "motivated"];
const ZODIAC = ["Aries", "Taurus", "Gemini", "Cancer", "Leo", "Virgo", "Libra", "Scorpio", "Sagittarius", "Capricorn", "Aquarius", "Pisces"];
const STATUS_OPTIONS = ["Single", "In a relationship", "Married", "Divorced", "Swinger", "It's complicated"];

const EditProfilePage: React.FC = () => {
  const { currentUser, updateProfile, loading } = useSocialStore();
  const navigate = useNavigate();

  const [form, setForm] = useState({
    displayName: currentUser?.displayName ?? "",
    avatarUrl: currentUser?.avatarUrl ?? "",
    headline: currentUser?.headline ?? "",
    aboutMe: currentUser?.aboutMe ?? "",
    whoIdLikeToMeet: currentUser?.whoIdLikeToMeet ?? "",
    interests: currentUser?.interests ?? "",
    musicInterests: currentUser?.musicInterests ?? "",
    movieInterests: currentUser?.movieInterests ?? "",
    profileCss: currentUser?.profileCss ?? "",
    profileBgUrl: currentUser?.profileBgUrl ?? "",
    mood: currentUser?.mood ?? "happy",
    gender: currentUser?.gender ?? "",
    age: currentUser?.age ?? 0,
    location: currentUser?.location ?? "",
    orientation: currentUser?.orientation ?? "",
    status: currentUser?.status ?? "Single",
    bodyType: currentUser?.bodyType ?? "",
    ethnicity: currentUser?.ethnicity ?? "",
    zodiacSign: currentUser?.zodiacSign ?? "",
    smokeDrink: currentUser?.smokeDrink ?? "",
    children: currentUser?.children ?? "",
    education: currentUser?.education ?? "",
    occupation: currentUser?.occupation ?? "",
    income: currentUser?.income ?? "",
    heroSongPath: currentUser?.heroSongPath ?? "",
    heroSongTitle: currentUser?.heroSongTitle ?? "",
  });

  const set = (key: string, value: string | number) => setForm(prev => ({ ...prev, [key]: value }));

  const handleSave = async () => {
    await updateProfile({
      displayName: form.displayName || undefined,
      avatarUrl: form.avatarUrl || undefined,
      headline: form.headline || undefined,
      aboutMe: form.aboutMe || undefined,
      whoIdLikeToMeet: form.whoIdLikeToMeet || undefined,
      interests: form.interests || undefined,
      musicInterests: form.musicInterests || undefined,
      movieInterests: form.movieInterests || undefined,
      profileCss: form.profileCss ?? undefined,
      profileBgUrl: form.profileBgUrl || undefined,
      mood: form.mood || undefined,
      gender: form.gender || undefined,
      age: form.age || undefined,
      location: form.location || undefined,
      orientation: form.orientation || undefined,
      status: form.status || undefined,
      bodyType: form.bodyType || undefined,
      ethnicity: form.ethnicity || undefined,
      zodiacSign: form.zodiacSign || undefined,
      smokeDrink: form.smokeDrink || undefined,
      children: form.children || undefined,
      education: form.education || undefined,
      occupation: form.occupation || undefined,
      income: form.income || undefined,
      heroSongPath: form.heroSongPath || undefined,
      heroSongTitle: form.heroSongTitle || undefined,
    });
    navigate("/social/profile");
  };

  return (
    <div style={{ maxWidth: 700 }}>
      <h1 className="social-page-title">Edit Profile</h1>

      {/* Basic Info */}
      <div className="social-card">
        <div className="social-card-header">Basic Info</div>
        <div className="social-form-group">
          <label className="social-label">Display Name</label>
          <input className="social-input" value={form.displayName} onChange={e => set("displayName", e.target.value)} />
        </div>
        <div className="social-form-group">
          <label className="social-label">Headline (tagline)</label>
          <input className="social-input" placeholder="e.g. Living life to the fullest 🌟" value={form.headline} onChange={e => set("headline", e.target.value)} />
        </div>
        <div className="social-form-group">
          <label className="social-label">Avatar URL</label>
          <input className="social-input" placeholder="https://..." value={form.avatarUrl} onChange={e => set("avatarUrl", e.target.value)} />
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.75rem" }}>
          <div className="social-form-group">
            <label className="social-label">Gender</label>
            <input className="social-input" value={form.gender} onChange={e => set("gender", e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Age</label>
            <input className="social-input" type="number" value={form.age || ""} onChange={e => set("age", parseInt(e.target.value) || 0)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Location</label>
            <input className="social-input" placeholder="City, State" value={form.location} onChange={e => set("location", e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Orientation</label>
            <input className="social-input" value={form.orientation} onChange={e => set("orientation", e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Status</label>
            <select className="social-select" style={{ width: "100%" }} value={form.status} onChange={e => set("status", e.target.value)}>
              {STATUS_OPTIONS.map(s => <option key={s} value={s}>{s}</option>)}
            </select>
          </div>
          <div className="social-form-group">
            <label className="social-label">Zodiac Sign</label>
            <select className="social-select" style={{ width: "100%" }} value={form.zodiacSign} onChange={e => set("zodiacSign", e.target.value)}>
              <option value="">Select...</option>
              {ZODIAC.map(z => <option key={z} value={z}>{z}</option>)}
            </select>
          </div>
          <div className="social-form-group">
            <label className="social-label">Occupation</label>
            <input className="social-input" value={form.occupation} onChange={e => set("occupation", e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Education</label>
            <input className="social-input" value={form.education} onChange={e => set("education", e.target.value)} />
          </div>
        </div>
        <div className="social-form-group">
          <label className="social-label">Current Mood</label>
          <select className="social-select" style={{ width: "100%" }} value={form.mood} onChange={e => set("mood", e.target.value)}>
            {MOODS.map(m => <option key={m} value={m}>{m}</option>)}
          </select>
        </div>
      </div>

      {/* About Me */}
      <div className="social-card">
        <div className="social-card-header">About Me</div>
        <div className="social-form-group">
          <label className="social-label">About Me (HTML/text allowed, just like MySpace!)</label>
          <textarea className="social-textarea" rows={6} value={form.aboutMe} onChange={e => set("aboutMe", e.target.value)}
            placeholder="Tell the world about yourself..." />
        </div>
        <div className="social-form-group">
          <label className="social-label">Who I'd Like to Meet</label>
          <textarea className="social-textarea" rows={3} value={form.whoIdLikeToMeet} onChange={e => set("whoIdLikeToMeet", e.target.value)} />
        </div>
      </div>

      {/* Interests */}
      <div className="social-card">
        <div className="social-card-header">Interests</div>
        <div className="social-form-group">
          <label className="social-label">General Interests</label>
          <textarea className="social-textarea" rows={2} value={form.interests} onChange={e => set("interests", e.target.value)} />
        </div>
        <div className="social-form-group">
          <label className="social-label">Music</label>
          <textarea className="social-textarea" rows={2} value={form.musicInterests} onChange={e => set("musicInterests", e.target.value)} />
        </div>
        <div className="social-form-group">
          <label className="social-label">Movies / TV</label>
          <textarea className="social-textarea" rows={2} value={form.movieInterests} onChange={e => set("movieInterests", e.target.value)} />
        </div>
      </div>

      {/* Profile Song */}
      <div className="social-card">
        <div className="social-card-header">🎵 Profile Song</div>
        <div className="social-form-group">
          <label className="social-label">Profile Song Title</label>
          <input className="social-input" value={form.heroSongTitle} onChange={e => set("heroSongTitle", e.target.value)} />
        </div>
        <div className="social-form-group">
          <label className="social-label">Profile Song File/URL</label>
          <input className="social-input" value={form.heroSongPath} onChange={e => set("heroSongPath", e.target.value)} />
        </div>
      </div>

      {/* Customize Profile (CSS!) */}
      <div className="social-card">
        <div className="social-card-header">🎨 Customize Your Profile</div>
        <p style={{ fontSize: "0.75rem", color: "#999", marginBottom: "0.5rem" }}>
          Just like the OG MySpace — paste your custom CSS to style your profile page!
        </p>
        <div className="social-form-group">
          <label className="social-label">Custom CSS</label>
          <textarea
            className="social-textarea"
            rows={8}
            style={{ fontFamily: "monospace", fontSize: "0.8rem" }}
            value={form.profileCss}
            onChange={e => set("profileCss", e.target.value)}
            placeholder={`.profile-custom-css {\n  background: url('...');\n  color: hotpink;\n}\n.profile-name {\n  font-family: 'Comic Sans MS';\n  color: lime;\n}`}
          />
        </div>
        <div className="social-form-group">
          <label className="social-label">Background Image URL</label>
          <input className="social-input" value={form.profileBgUrl} onChange={e => set("profileBgUrl", e.target.value)}
            placeholder="https://..." />
        </div>
      </div>

      {/* Save */}
      <div style={{ display: "flex", gap: "0.75rem", marginTop: "1rem" }}>
        <button className="social-btn" onClick={handleSave} disabled={loading}>
          {loading ? "Saving..." : "Save Changes"}
        </button>
        <button className="social-btn social-btn-outline" onClick={() => navigate("/social/profile")}>
          Cancel
        </button>
      </div>
    </div>
  );
};

export default EditProfilePage;
