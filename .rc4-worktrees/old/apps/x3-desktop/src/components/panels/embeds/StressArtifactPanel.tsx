import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/blockchain-stress-test.html"
  : "https://x3star.net/blockchain-stress-test.html";

const StressArtifactPanel: React.FC = () => (
  <IframePanel url={URL} title="StressArtifact" />
);

export default StressArtifactPanel;
