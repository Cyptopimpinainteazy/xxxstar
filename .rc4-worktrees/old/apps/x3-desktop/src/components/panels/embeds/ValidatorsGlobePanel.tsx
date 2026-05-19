import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:5177";

const ValidatorsGlobePanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="Validator Globe" />
);

export default ValidatorsGlobePanel;
