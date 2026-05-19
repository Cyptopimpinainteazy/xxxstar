import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-if-you-invested.html"
  : "https://x3star.net/x3star-if-you-invested.html";

const IfYouInvestedPanel: React.FC = () => (
  <IframePanel url={URL} title="IfYouInvested" />
);

export default IfYouInvestedPanel;
