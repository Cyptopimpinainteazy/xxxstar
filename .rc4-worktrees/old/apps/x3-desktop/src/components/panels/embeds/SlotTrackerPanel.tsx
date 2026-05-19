import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-slot-tracker.html"
  : "https://x3star.net/x3star-slot-tracker.html";

const SlotTrackerPanel: React.FC = () => (
  <IframePanel url={URL} title="SlotTracker" />
);

export default SlotTrackerPanel;
