import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:8080/dashboard.html";

const SwarmAutonomicPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Swarm Autonomic" />
);

export default SwarmAutonomicPanel;
