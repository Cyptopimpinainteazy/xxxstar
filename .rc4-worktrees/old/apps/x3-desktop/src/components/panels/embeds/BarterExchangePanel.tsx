import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-barter-exchange.html"
  : "https://x3star.net/x3star-barter-exchange.html";

const BarterExchangePanel: React.FC = () => (
  <IframePanel url={URL} title="BarterExchange" />
);

export default BarterExchangePanel;
