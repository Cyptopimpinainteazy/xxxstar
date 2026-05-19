import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-leaderboard-arena.html"
  : "https://x3star.net/x3star-leaderboard-arena.html";

const LeaderboardArenaPanel: React.FC = () => (
  <IframePanel url={URL} title="LeaderboardArena" />
);

export default LeaderboardArenaPanel;
