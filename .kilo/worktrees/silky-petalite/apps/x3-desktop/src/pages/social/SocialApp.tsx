import React, { useEffect } from "react";
import { useSocialStore } from "@/stores/socialStore";
import AuthPage from "./AuthPage";
import SocialShell from "./SocialShell";
import "@/styles/social.css";

const SocialApp: React.FC = () => {
  const { isLoggedIn, restoreSession } = useSocialStore();

  useEffect(() => {
    restoreSession();
  }, []);

  if (!isLoggedIn) return <AuthPage />;
  return <SocialShell />;
};

export default SocialApp;
