import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-ecosystem-heartbeat.html"
  : "https://x3star.net/x3star-ecosystem-heartbeat.html";

const EcosystemHeartbeatPanel: React.FC = () => (
  <IframePanel url={URL} title="EcosystemHeartbeat" />
);

export default EcosystemHeartbeatPanel;
