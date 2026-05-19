import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-competitor-graveyard.html"
  : "https://x3star.net/x3star-competitor-graveyard.html";

const CompetitorGraveyardPanel: React.FC = () => (
  <IframePanel url={URL} title="CompetitorGraveyard" />
);

export default CompetitorGraveyardPanel;
