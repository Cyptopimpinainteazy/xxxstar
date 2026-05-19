import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:3020";

const TpsMonitorPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="TPS Monitor" />
);

export default TpsMonitorPanel;
