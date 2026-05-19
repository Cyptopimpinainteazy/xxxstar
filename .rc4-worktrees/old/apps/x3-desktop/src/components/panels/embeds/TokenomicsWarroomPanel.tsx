import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-tokenomics-warroom.html"
  : "https://x3star.net/x3star-tokenomics-warroom.html";

const TokenomicsWarroomPanel: React.FC = () => (
  <IframePanel url={URL} title="TokenomicsWarroom" />
);

export default TokenomicsWarroomPanel;
