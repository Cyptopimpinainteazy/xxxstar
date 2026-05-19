import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-social-proof-wall.html"
  : "https://x3star.net/x3star-social-proof-wall.html";

const SocialProofWallPanel: React.FC = () => (
  <IframePanel url={URL} title="SocialProofWall" />
);

export default SocialProofWallPanel;
