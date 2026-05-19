import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:5178";

const X3IntelligenceFullPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="X3 Intelligence" />
);

export default X3IntelligenceFullPanel;
