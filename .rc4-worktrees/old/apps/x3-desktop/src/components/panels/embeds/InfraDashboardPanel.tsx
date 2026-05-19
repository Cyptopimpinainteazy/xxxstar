import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:5175";

const InfraDashboardPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Infra Dashboard" />
);

export default InfraDashboardPanel;
