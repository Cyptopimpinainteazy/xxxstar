import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-mission-terminal.html"
  : "https://x3star.net/x3star-mission-terminal.html";

const MissionTerminalPanel: React.FC = () => (
  <IframePanel url={URL} title="MissionTerminal" />
);

export default MissionTerminalPanel;
