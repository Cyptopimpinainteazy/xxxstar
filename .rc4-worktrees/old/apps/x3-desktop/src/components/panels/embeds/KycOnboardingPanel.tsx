import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "http://localhost:8080/x3star-kyc-onboarding.html"
  : "https://x3star.net/x3star-kyc-onboarding.html";

const KycOnboardingPanel: React.FC = () => (
  <IframePanel url={URL} title="KycOnboarding" />
);

export default KycOnboardingPanel;
