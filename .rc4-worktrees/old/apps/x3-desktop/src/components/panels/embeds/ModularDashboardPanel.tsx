import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:5176";

const ModularDashboardPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Modular Dashboard" />
);

export default ModularDashboardPanel;
