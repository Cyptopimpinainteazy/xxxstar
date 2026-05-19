import React, { Suspense, useEffect } from "react";
import { Routes, Route, Link, useNavigate } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";

const SystemCommandPanel = React.lazy(() => import("../admin/SystemCommandPanel"));
import HomePage from "./HomePage";
import ProfilePage from "./ProfilePage";
import EditProfilePage from "./EditProfilePage";
import FriendsPage from "./FriendsPage";
import MessagesPage from "./MessagesPage";
import BulletinsPage from "./BulletinsPage";
import BrowsePage from "./BrowsePage";
import BlogPage from "./BlogPage";
import PhotosPage from "./PhotosPage";
import MusicPage from "./MusicPage";
import GroupsPage from "./GroupsPage";
import SearchPage from "./SearchPage";
import ViewProfilePage from "./ViewProfilePage";

const ROLE_BADGE: Record<string, { icon: string; color: string; label: string }> = {
  team:  { icon: "🔶", color: "#ff6b35", label: "Team" },
  admin: { icon: "👑", color: "#ff2d55", label: "Admin" },
  vip:   { icon: "💎", color: "#a855f7", label: "VIP" },
};

const SocialShell: React.FC = () => {
  const { session, currentUser, stats, logout, loadStats } = useSocialStore();
  const navigate = useNavigate();

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 30000);
    return () => clearInterval(interval);
  }, [loadStats]);

  const handleLogout = async () => {
    await logout();
    navigate("/social");
  };

  return (
    <div className="social-app" style={{ overflowY: "auto", height: "100vh" }}>
      {/* Global Back to Desktop Button */}
      <button 
        onClick={() => navigate("/")}
        className="fixed top-4 left-4 z-[9999] flex items-center gap-2 px-4 py-2 bg-[#ff6b35] text-white rounded-full shadow-lg hover:bg-[#ff8c42] transition-colors font-medium text-sm"
        style={{ boxShadow: "0 4px 12px rgba(255, 107, 53, 0.3)" }}
      >
        <span className="text-lg leading-none mb-[2px]">←</span> Back to Desktop
      </button>

      {/* KING Admin Panel (only for King in KING team) */}
      {currentUser?.username === "King" && currentUser?.role === "admin" && (
        <div className="king-admin-panel" style={{background:'#222',color:'#ffd740',padding:'8px',margin:'8px 0',borderRadius:'8px',border:'2px solid #ffd740',textAlign:'center'}}>
          <h2 style={{marginBottom:'8px'}}>KING Admin Panel</h2>
          <div style={{display:'flex',flexWrap:'wrap',gap:'8px',justifyContent:'center'}}>
            <button style={{background:'#ff2d55',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/users')}>User Management</button>
            <button style={{background:'#ff6b35',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/teams')}>Team Settings</button>
            <button style={{background:'#a855f7',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/site')}>Site Tools</button>
            <button style={{background:'#11a0dc',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/ban')}>Ban Controls</button>
            <button style={{background:'#ffd740',color:'#222',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/logs')}>Logs & Audits</button>
            <button style={{background:'#64b5f6',color:'#222',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/settings')}>System Settings</button>
            <button style={{background:'#ef5350',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/crm')}>CRM Admin</button>
            <button style={{background:'#ff8c42',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/analytics')}>Analytics</button>
            <button style={{background:'#78909c',color:'#fff',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold'}} onClick={()=>navigate('/admin/devtools')}>Dev Tools</button>
            <button style={{background:'#222',color:'#ffd740',padding:'6px 16px',borderRadius:'6px',fontWeight:'bold',border:'1px solid #ffd740'}} onClick={()=>navigate('/admin/commands')}>System Commands</button>
          </div>
        </div>
      )}
      {/* Navigation Bar */}
      <nav className="social-nav">
        <Link to="/" className="social-nav-back" style={{ textDecoration: "none", color: "#ff6b35", fontWeight: 700, fontSize: "0.85rem", marginRight: "8px", display: "flex", alignItems: "center", gap: "4px" }}>
          ← Desktop
        </Link>
        <Link to="/social" className="social-nav-logo" style={{ textDecoration: "none" }}>
          AtlasSpace
        </Link>
        <div className="social-nav-links">
          <Link to="/social">Home</Link>
          <Link to="/social/profile">My Profile</Link>
          <Link to="/social/friends">
            Friends
            {(stats?.pendingRequests ?? 0) > 0 && (
              <span className="nav-badge">{stats!.pendingRequests}</span>
            )}
          </Link>
          <Link to="/social/messages">
            Mail
            {(stats?.unreadMessages ?? 0) > 0 && (
              <span className="nav-badge">{stats!.unreadMessages}</span>
            )}
          </Link>
          <Link to="/social/bulletins">Bulletins</Link>
          <Link to="/social/blog">Blog</Link>
          <Link to="/social/photos">Photos</Link>
          <Link to="/social/music">Music</Link>
          <Link to="/social/groups">Groups</Link>
          <Link to="/social/browse">Browse</Link>
          <Link to="/social/search">Search</Link>
          <Link to="/crm" style={{ color: "#ff6b35" }}>📅 CRM</Link>
          <button onClick={handleLogout}>Sign Out</button>
          <span style={{ color: "#666", fontSize: "0.7rem", padding: "0.4rem", display: "flex", alignItems: "center", gap: "4px" }}>
            {currentUser?.role && ROLE_BADGE[currentUser.role] && (
              <span style={{
                background: `${ROLE_BADGE[currentUser.role].color}22`,
                border: `1px solid ${ROLE_BADGE[currentUser.role].color}55`,
                color: ROLE_BADGE[currentUser.role].color,
                borderRadius: 10, padding: "1px 6px", fontSize: "0.6rem", fontWeight: 700,
              }}>
                {ROLE_BADGE[currentUser.role].icon} {ROLE_BADGE[currentUser.role].label}
              </span>
            )}
            {currentUser?.displayName ?? session?.username}
          </span>
        </div>
      </nav>

      {/* Page Content */}
      <div className="social-content">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/profile" element={<ProfilePage />} />
          <Route path="/profile/edit" element={<EditProfilePage />} />
          <Route path="/friends" element={<FriendsPage />} />
          <Route path="/messages" element={<MessagesPage />} />
          <Route path="/bulletins" element={<BulletinsPage />} />
          <Route path="/blog" element={<BlogPage />} />
          <Route path="/photos" element={<PhotosPage />} />
          <Route path="/music" element={<MusicPage />} />
          <Route path="/groups" element={<GroupsPage />} />
          <Route path="/browse" element={<BrowsePage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/view/:userId" element={<ViewProfilePage />} />
          <Route path="/admin/commands" element={<Suspense fallback={<div>Loading...</div>}><SystemCommandPanel /></Suspense>} />
        </Routes>
      </div>
    </div>
  );
};

export default SocialShell;
