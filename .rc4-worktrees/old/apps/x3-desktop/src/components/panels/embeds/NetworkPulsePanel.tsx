import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-network-pulse.html"
  : "https://x3star.net/x3star-network-pulse.html";

const NetworkPulsePanel: React.FC = () => (
  <IframePanel url={URL} title="NetworkPulse" />
);

export default NetworkPulsePanel;
