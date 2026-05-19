import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-operator-war-room.html"
  : "https://x3star.net/x3star-operator-war-room.html";

const OperatorWarRoomPanel: React.FC = () => (
  <IframePanel url={URL} title="OperatorWarRoom" />
);

export default OperatorWarRoomPanel;
