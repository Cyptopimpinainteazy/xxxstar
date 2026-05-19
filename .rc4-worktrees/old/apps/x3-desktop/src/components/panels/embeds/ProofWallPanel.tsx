import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-proof-wall.html"
  : "https://x3star.net/x3star-proof-wall.html";

const ProofWallPanel: React.FC = () => (
  <IframePanel url={URL} title="ProofWall" />
);

export default ProofWallPanel;
