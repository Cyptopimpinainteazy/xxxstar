import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-fundraise-thermometer.html"
  : "https://x3star.net/x3star-fundraise-thermometer.html";

const FundraiseThermometerPanel: React.FC = () => (
  <IframePanel url={URL} title="FundraiseThermometer" />
);

export default FundraiseThermometerPanel;
