import React, { useEffect, useState } from "react";
import { useSocialStore } from "@/stores/socialStore";
import { useNavigate } from "react-router-dom";
import CrmShell from "./CrmShell";
import "@/styles/crm.css";

const CrmApp: React.FC = () => {
  const { isLoggedIn, restoreSession } = useSocialStore();
  const navigate = useNavigate();
  const [sessionChecked, setSessionChecked] = useState(false);

  useEffect(() => {
    restoreSession();
    setSessionChecked(true);
  }, []);

  useEffect(() => {
    if (sessionChecked && !isLoggedIn) {
      navigate("/social", { replace: true });
    }
  }, [isLoggedIn, navigate, sessionChecked]);

  if (!sessionChecked || !isLoggedIn) return null;
  return <CrmShell />;
};

export default CrmApp;
