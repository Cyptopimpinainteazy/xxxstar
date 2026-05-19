import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-governance.html"
  : "https://x3star.net/x3star-governance.html";

const GovernancePanel: React.FC = () => (
  <IframePanel url={URL} title="Governance" />
);

export default GovernancePanel;
