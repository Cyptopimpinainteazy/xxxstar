import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-affiliate.html"
  : "https://x3star.net/x3star-affiliate.html";

const AffiliatePanel: React.FC = () => (
  <IframePanel url={URL} title="Affiliate" />
);

export default AffiliatePanel;
