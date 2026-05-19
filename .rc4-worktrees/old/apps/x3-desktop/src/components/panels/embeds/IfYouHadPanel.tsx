import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-if-you-had.html"
  : "https://x3star.net/x3star-if-you-had.html";

const IfYouHadPanel: React.FC = () => (
  <IframePanel url={URL} title="IfYouHad" />
);

export default IfYouHadPanel;
