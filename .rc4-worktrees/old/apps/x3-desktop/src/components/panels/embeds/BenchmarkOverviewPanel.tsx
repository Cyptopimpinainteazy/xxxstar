import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/chainbench-ultimate.html"
  : "https://x3star.net/chainbench-ultimate.html";

const BenchmarkOverviewPanel: React.FC = () => (
  <IframePanel url={URL} title="BenchmarkOverview" />
);

export default BenchmarkOverviewPanel;
