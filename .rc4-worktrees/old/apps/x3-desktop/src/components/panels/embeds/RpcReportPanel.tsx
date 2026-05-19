import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/chainbench-pro.html"
  : "https://x3star.net/chainbench-pro.html";

const RpcReportPanel: React.FC = () => (
  <IframePanel url={URL} title="RpcReport" />
);

export default RpcReportPanel;
