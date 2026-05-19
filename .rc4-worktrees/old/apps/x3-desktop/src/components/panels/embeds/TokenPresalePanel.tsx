import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-token-presale.html"
  : "https://x3star.net/x3star-token-presale.html";

const TokenPresalePanel: React.FC = () => (
  <IframePanel url={URL} title="TokenPresale" />
);

export default TokenPresalePanel;
