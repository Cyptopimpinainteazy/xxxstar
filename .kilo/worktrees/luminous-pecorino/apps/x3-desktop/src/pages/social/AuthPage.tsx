import React, { useState, useCallback } from "react";
import { useSocialStore } from "@/stores/socialStore";
import { validateTeamCode } from "@/services/socialService";

const TIER_INFO: Record<string, { label: string; color: string; icon: string; desc: string }> = {
  user:  { label: "Public",       color: "#888",    icon: "🌐", desc: "Standard access — browse, post, connect" },
  team:  { label: "Team",         color: "#ff6b35", icon: "🔶", desc: "Project team — full access + team tools" },
  admin: { label: "Admin",        color: "#ff2d55", icon: "👑", desc: "Administrator — full control + code management" },
  vip:   { label: "VIP",          color: "#a855f7", icon: "💎", desc: "VIP member — exclusive badge + priority" },
};

const AuthPage: React.FC = () => {
  const [tab, setTab] = useState<"login" | "register">("login");
  const { login, register, loading, error } = useSocialStore();

  // Login form
  const [loginUser, setLoginUser] = useState("");
  const [loginPass, setLoginPass] = useState("");

  // Register form
  const [regUser, setRegUser] = useState("");
  const [regName, setRegName] = useState("");
  const [regEmail, setRegEmail] = useState("");
  const [regPass, setRegPass] = useState("");
  const [regPass2, setRegPass2] = useState("");
  const [teamCode, setTeamCode] = useState("");
  const [showTeamCode, setShowTeamCode] = useState(false);
  const [codeStatus, setCodeStatus] = useState<{ valid: boolean; label: string } | null>(null);
  const [codeChecking, setCodeChecking] = useState(false);
  const [localError, setLocalError] = useState("");

  const checkCode = useCallback(async (code: string) => {
    if (!code.trim()) { setCodeStatus(null); return; }
    setCodeChecking(true);
    try {
      const label = await validateTeamCode(code.trim());
      setCodeStatus({ valid: true, label });
    } catch {
      setCodeStatus({ valid: false, label: "Invalid code" });
    }
    setCodeChecking(false);
  }, []);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setLocalError("");
    if (!loginUser || !loginPass) { setLocalError("Fill in all fields"); return; }
    await login(loginUser, loginPass);
  };

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    setLocalError("");
    if (!regUser || !regName || !regEmail || !regPass) { setLocalError("Fill in all fields"); return; }
    if (regPass !== regPass2) { setLocalError("Passwords don't match"); return; }
    if (regPass.length < 4) { setLocalError("Password must be at least 4 characters"); return; }
    if (showTeamCode && teamCode.trim() && codeStatus && !codeStatus.valid) {
      setLocalError("Please enter a valid team code or remove it");
      return;
    }
    await register(regUser, regName, regEmail, regPass, showTeamCode ? teamCode.trim() || undefined : undefined);
  };

  return (
    <div className="auth-page">
      <div className="auth-card" style={{ maxWidth: 440 }}>
        <div className="auth-logo">
          <h1>AtlasSpace</h1>
          <p>A place for friends.</p>
        </div>

        {/* Tier badges display */}
        <div style={{ display: "flex", gap: "0.5rem", justifyContent: "center", marginBottom: "1rem", flexWrap: "wrap" }}>
          {Object.entries(TIER_INFO).map(([key, t]) => (
            <span key={key} style={{
              background: `${t.color}22`, border: `1px solid ${t.color}55`, color: t.color,
              borderRadius: 12, padding: "2px 10px", fontSize: "0.65rem", fontWeight: 600,
              display: "flex", alignItems: "center", gap: "3px",
            }}>
              {t.icon} {t.label}
            </span>
          ))}
        </div>

        <div className="auth-tabs">
          <button className={`auth-tab ${tab === "login" ? "active" : ""}`} onClick={() => setTab("login")}>
            Sign In
          </button>
          <button className={`auth-tab ${tab === "register" ? "active" : ""}`} onClick={() => setTab("register")}>
            Sign Up
          </button>
        </div>

        {(error || localError) && (
          <div className="auth-error">{localError || error}</div>
        )}

        {tab === "login" ? (
          <form onSubmit={handleLogin}>
            <div className="social-form-group">
              <label className="social-label">Username</label>
              <input className="social-input" value={loginUser} onChange={e => setLoginUser(e.target.value)} autoFocus />
            </div>
            <div className="social-form-group">
              <label className="social-label">Password</label>
              <input className="social-input" type="password" value={loginPass} onChange={e => setLoginPass(e.target.value)} />
            </div>
            <button className="social-btn" style={{ width: "100%" }} disabled={loading}>
              {loading ? "Signing in..." : "Sign In"}
            </button>
          </form>
        ) : (
          <form onSubmit={handleRegister}>
            <div className="social-form-group">
              <label className="social-label">Username</label>
              <input className="social-input" value={regUser} onChange={e => setRegUser(e.target.value)} autoFocus />
            </div>
            <div className="social-form-group">
              <label className="social-label">Display Name</label>
              <input className="social-input" value={regName} onChange={e => setRegName(e.target.value)} />
            </div>
            <div className="social-form-group">
              <label className="social-label">Email</label>
              <input className="social-input" type="email" value={regEmail} onChange={e => setRegEmail(e.target.value)} />
            </div>
            <div className="social-form-group">
              <label className="social-label">Password</label>
              <input className="social-input" type="password" value={regPass} onChange={e => setRegPass(e.target.value)} />
            </div>
            <div className="social-form-group">
              <label className="social-label">Confirm Password</label>
              <input className="social-input" type="password" value={regPass2} onChange={e => setRegPass2(e.target.value)} />
            </div>

            {/* Team Code Section */}
            <div style={{
              borderTop: "1px solid #333", marginTop: "0.75rem", paddingTop: "0.75rem",
            }}>
              <button type="button" onClick={() => setShowTeamCode(!showTeamCode)} style={{
                background: "none", border: "none", color: "#ff6b35", cursor: "pointer",
                fontSize: "0.8rem", padding: 0, textDecoration: "underline", fontWeight: 600,
              }}>
                {showTeamCode ? "▾ Hide team code" : "▸ Have a team invite code?"}
              </button>

              {showTeamCode && (
                <div style={{ marginTop: "0.5rem" }}>
                  <div style={{
                    background: "#1a1a2e", border: "1px solid #ff6b3533", borderRadius: 8,
                    padding: "0.75rem", marginBottom: "0.5rem",
                  }}>
                    <div style={{ fontSize: "0.7rem", color: "#999", marginBottom: "0.5rem" }}>
                      🔐 Enter your invite code to join as a project team member
                    </div>
                    <div style={{ display: "flex", gap: "0.5rem" }}>
                      <input
                        className="social-input"
                        style={{ flex: 1, fontFamily: "monospace", letterSpacing: 1, textTransform: "uppercase" }}
                        placeholder="X3-XXXX-XXXX"
                        value={teamCode}
                        onChange={e => {
                          setTeamCode(e.target.value.toUpperCase());
                          setCodeStatus(null);
                        }}
                        onBlur={() => checkCode(teamCode)}
                      />
                      <button type="button" className="social-btn social-btn-sm" onClick={() => checkCode(teamCode)}
                        disabled={codeChecking || !teamCode.trim()} style={{ whiteSpace: "nowrap" }}>
                        {codeChecking ? "..." : "Verify"}
                      </button>
                    </div>

                    {codeStatus && (
                      <div style={{
                        marginTop: "0.5rem", fontSize: "0.75rem", fontWeight: 600,
                        color: codeStatus.valid ? "#4ade80" : "#f87171",
                        display: "flex", alignItems: "center", gap: "0.25rem",
                      }}>
                        {codeStatus.valid ? "✓" : "✗"} {codeStatus.label}
                      </div>
                    )}
                  </div>

                  <div style={{ fontSize: "0.65rem", color: "#666" }}>
                    No code? No problem — sign up as a public user and request access later.
                  </div>
                </div>
              )}
            </div>

            <button className="social-btn" style={{ width: "100%", marginTop: "0.75rem" }} disabled={loading}>
              {loading ? "Creating account..." : (
                showTeamCode && teamCode.trim() && codeStatus?.valid
                  ? `Sign Up as ${codeStatus.label}`
                  : "Sign Up Free"
              )}
            </button>
          </form>
        )}
      </div>
    </div>
  );
};

export default AuthPage;
