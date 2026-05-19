import { Routes, Route, NavLink } from "react-router-dom";
import { useState } from "react";
import { FloorDashboard } from "./pages/FloorDashboard";
import { IntentsPage } from "./pages/IntentsPage";
import { AgentsPage } from "./pages/AgentsPage";
import { SlashingPage } from "./pages/SlashingPage";
import { ProofExplorer } from "./pages/ProofExplorer";
import { FloorRules } from "./pages/FloorRules";
import { BondsPage } from "./pages/BondsPage";
import { GuidePage } from "./pages/GuidePage";
import { WhyPage } from "./pages/WhyPage";
import HelpModal from "./components/HelpModal";
import { DemoDataBanner } from "./components/DemoDataBanner";

export function App() {
  const [showHelp, setShowHelp] = useState(false);

  return (
    <>
      {/* Data integrity alert — renders only when demo/fallback data is active */}
      <DemoDataBanner />
      <nav className="nav">
        <span className="nav-brand">X3</span>
        <NavLink to="/" end className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Floor
        </NavLink>
        <NavLink to="/intents" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Intents
        </NavLink>
        <NavLink to="/agents" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Agents
        </NavLink>
        <NavLink to="/slashing" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Slashing
        </NavLink>
        <NavLink to="/proofs" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Proofs
        </NavLink>
        <NavLink to="/bonds" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Bonds
        </NavLink>
        <NavLink to="/rules" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Rules
        </NavLink>
        <NavLink to="/guide" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          How to Use
        </NavLink>
        <NavLink to="/why" className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
          Why
        </NavLink>

        <button className="help-button" onClick={() => setShowHelp(true)} title="Quick help">
          ?
        </button>
      </nav>

      <Routes>
        <Route path="/" element={<FloorDashboard />} />
        <Route path="/intents" element={<IntentsPage />} />
        <Route path="/agents" element={<AgentsPage />} />
        <Route path="/slashing" element={<SlashingPage />} />
        <Route path="/proofs" element={<ProofExplorer />} />
        <Route path="/bonds" element={<BondsPage />} />
        <Route path="/rules" element={<FloorRules />} />
        <Route path="/guide" element={<GuidePage />} />
        <Route path="/why" element={<WhyPage />} />
      </Routes>

      <HelpModal open={showHelp} onClose={() => setShowHelp(false)} />
    </>
  );
}
