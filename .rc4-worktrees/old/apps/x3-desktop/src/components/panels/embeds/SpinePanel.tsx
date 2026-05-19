import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-spine.html"
  : "https://x3star.net/x3star-spine.html";

const SpinePanel: React.FC = () => (
  <IframePanel url={URL} title="Spine" />
);

export default SpinePanel;
