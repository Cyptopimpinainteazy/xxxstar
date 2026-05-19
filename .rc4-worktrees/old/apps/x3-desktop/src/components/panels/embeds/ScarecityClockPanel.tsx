import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-scarcity-clock.html"
  : "https://x3star.net/x3star-scarcity-clock.html";

const ScarecityClockPanel: React.FC = () => (
  <IframePanel url={URL} title="ScarecityClock" />
);

export default ScarecityClockPanel;
