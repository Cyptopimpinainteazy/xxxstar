import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-grant-hub.html"
  : "https://x3star.net/x3star-grant-hub.html";

const GrantHubPanel: React.FC = () => (
  <IframePanel url={URL} title="GrantHub" />
);

export default GrantHubPanel;
