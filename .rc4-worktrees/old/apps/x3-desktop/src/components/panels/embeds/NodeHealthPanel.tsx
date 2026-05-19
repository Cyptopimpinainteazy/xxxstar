import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-node-health.html"
  : "https://x3star.net/x3star-node-health.html";

const NodeHealthPanel: React.FC = () => (
  <IframePanel url={URL} title="NodeHealth" />
);

export default NodeHealthPanel;
