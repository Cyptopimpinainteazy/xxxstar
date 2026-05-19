import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-transparency-ledger.html"
  : "https://x3star.net/x3star-transparency-ledger.html";

const TransparencyLedgerPanel: React.FC = () => (
  <IframePanel url={URL} title="TransparencyLedger" />
);

export default TransparencyLedgerPanel;
