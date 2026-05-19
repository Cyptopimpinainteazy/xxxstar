import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const DEV_URL = "http://localhost:3001";

const WalletAppPanel: React.FC = () => (
  <IframePanel url={DEV_URL} title="X3 Wallet" />
);

export default WalletAppPanel;
