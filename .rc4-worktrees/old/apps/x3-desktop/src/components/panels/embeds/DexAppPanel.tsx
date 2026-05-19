import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:3002";

const DexAppPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="X3 DEX" />
);

export default DexAppPanel;
