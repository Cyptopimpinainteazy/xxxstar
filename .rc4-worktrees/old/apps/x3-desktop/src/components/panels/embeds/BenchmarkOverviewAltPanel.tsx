import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/chainbench-ultimate(1).html"
  : "https://x3star.net/chainbench-ultimate(1).html";

const BenchmarkOverviewAltPanel: React.FC = () => (
  <IframePanel url={URL} title="BenchmarkOverviewAlt" />
);

export default BenchmarkOverviewAltPanel;
