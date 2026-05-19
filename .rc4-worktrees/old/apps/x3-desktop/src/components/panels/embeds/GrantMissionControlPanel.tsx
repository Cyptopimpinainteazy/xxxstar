import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-grant-mission-control.html"
  : "https://x3star.net/x3star-grant-mission-control.html";

const GrantMissionControlPanel: React.FC = () => (
  <IframePanel url={URL} title="GrantMissionControl" />
);

export default GrantMissionControlPanel;
