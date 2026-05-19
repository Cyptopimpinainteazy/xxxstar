import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:5174";

const InfestructorDashboardPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Inferstructor Dashboard" />
);

export default InfestructorDashboardPanel;
