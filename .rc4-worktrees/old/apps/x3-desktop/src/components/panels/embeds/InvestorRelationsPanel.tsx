import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-investor-relations.html"
  : "https://x3star.net/x3star-investor-relations.html";

const InvestorRelationsPanel: React.FC = () => (
  <IframePanel url={URL} title="InvestorRelations" />
);

export default InvestorRelationsPanel;
