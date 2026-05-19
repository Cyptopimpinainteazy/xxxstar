import React from "react";
import IframePanel from "@/components/panels/IframePanel";

// In dev: use production server at :8080 (serves unified HTML pages)
// In prod: use https://x3star.net
const URL = import.meta.env.DEV
  ? "http://localhost:8080"
  : "https://x3star.net";

const X3FrontendPanel: React.FC = () => (
  <IframePanel url={URL} title="X3 Landing & Pages" />
);

export default X3FrontendPanel;
