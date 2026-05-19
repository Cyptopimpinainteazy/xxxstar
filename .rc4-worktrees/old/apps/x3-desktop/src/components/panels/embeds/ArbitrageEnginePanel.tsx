import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-arbitrage-engine.html"
  : "https://x3star.net/x3star-arbitrage-engine.html";

const ArbitrageEnginePanel: React.FC = () => (
  <IframePanel url={URL} title="ArbitrageEngine" />
);

export default ArbitrageEnginePanel;
