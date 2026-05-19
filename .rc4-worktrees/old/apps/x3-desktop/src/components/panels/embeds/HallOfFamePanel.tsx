import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-hall-of-fame.html"
  : "https://x3star.net/x3star-hall-of-fame.html";

const HallOfFamePanel: React.FC = () => (
  <IframePanel url={URL} title="HallOfFame" />
);

export default HallOfFamePanel;
