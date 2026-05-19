import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-dashboard.html"
  : "https://x3star.net/x3star-dashboard.html";

const DashboardPanel: React.FC = () => (
  <IframePanel url={URL} title="Dashboard" />
);

export default DashboardPanel;
