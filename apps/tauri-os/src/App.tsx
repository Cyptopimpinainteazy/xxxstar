import { useState } from "react";
import { SwarmCommand } from "./apps/SwarmCommand/SwarmCommand";
import { LiveTelemetryPanel } from "./components/LiveTelemetryPanel";

type View = "swarm" | "telemetry";

const navStyle: React.CSSProperties = {
  display: "flex",
  gap: 8,
  padding: "12px 24px",
  borderBottom: "1px solid #ddd",
  fontFamily: "Inter, sans-serif",
};

const buttonStyle: React.CSSProperties = {
  padding: "8px 16px",
  border: "1px solid #ccc",
  borderRadius: 8,
  background: "#fff",
  cursor: "pointer",
};

function App() {
  const [view, setView] = useState<View>("swarm");

  return (
    <>
      <nav style={navStyle}>
        <button
          style={{ ...buttonStyle, fontWeight: view === "swarm" ? 700 : 400 }}
          onClick={() => setView("swarm")}
        >
          Swarm Command
        </button>
        <button
          style={{ ...buttonStyle, fontWeight: view === "telemetry" ? 700 : 400 }}
          onClick={() => setView("telemetry")}
        >
          Live Telemetry
        </button>
      </nav>
      {view === "swarm" ? <SwarmCommand /> : <LiveTelemetryPanel />}
    </>
  );
}

export default App;
