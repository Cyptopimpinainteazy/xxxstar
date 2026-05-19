import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-portfolio.html"
  : "https://x3star.net/x3star-portfolio.html";

const PortfolioPanel: React.FC = () => (
  <IframePanel url={URL} title="Portfolio" />
);

export default PortfolioPanel;
