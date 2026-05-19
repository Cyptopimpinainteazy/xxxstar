import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-staking.html"
  : "https://x3star.net/x3star-staking.html";

const StakingPanel: React.FC = () => (
  <IframePanel url={URL} title="Staking" />
);

export default StakingPanel;
