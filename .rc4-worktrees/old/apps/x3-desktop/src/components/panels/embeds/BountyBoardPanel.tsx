import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-bounty-board.html"
  : "https://x3star.net/x3star-bounty-board.html";

const BountyBoardPanel: React.FC = () => (
  <IframePanel url={URL} title="BountyBoard" />
);

export default BountyBoardPanel;
