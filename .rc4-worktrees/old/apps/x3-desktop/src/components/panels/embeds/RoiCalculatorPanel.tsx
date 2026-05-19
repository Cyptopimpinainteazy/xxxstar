import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-roi-calculator.html"
  : "https://x3star.net/x3star-roi-calculator.html";

const RoiCalculatorPanel: React.FC = () => (
  <IframePanel url={URL} title="RoiCalculator" />
);

export default RoiCalculatorPanel;
