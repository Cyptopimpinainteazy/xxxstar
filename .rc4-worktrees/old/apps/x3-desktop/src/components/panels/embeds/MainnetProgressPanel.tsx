import React from "react";
import IframePanel from "@/components/panels/IframePanel";

// Static HTML — served from asset protocol in Tauri or a local HTTP server in dev.
// Run: cd web/mainnet-progress && python3 -m http.server 8181
const DEV_URL = "http://localhost:8181";

const MainnetProgressPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Mainnet Progress" />
);

export default MainnetProgressPanel;
