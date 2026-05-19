import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-whale-tracker.html"
  : "https://x3star.net/x3star-whale-tracker.html";

const WhaleTrackerPanel: React.FC = () => (
  <IframePanel url={URL} title="WhaleTracker" />
);

export default WhaleTrackerPanel;
