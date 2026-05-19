import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/blockchain-stress-test(1).html"
  : "https://x3star.net/blockchain-stress-test(1).html";

const StressArtifactAltPanel: React.FC = () => (
  <IframePanel url={URL} title="StressArtifactAlt" />
);

export default StressArtifactAltPanel;
