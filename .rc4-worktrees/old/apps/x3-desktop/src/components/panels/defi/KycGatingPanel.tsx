import React, { useState } from "react";
import { Shield, FileText, CheckCircle, AlertCircle, Fingerprint } from "lucide-react";
import clsx from "clsx";

interface KycLevel {
  level: number;
  name: string;
  requirements: string[];
  verified: boolean;
  limit: string;
}

export default function KycGatingPanel() {
  const [currentLevel, setCurrentLevel] = useState(1);
  const [kycStatus, setKycStatus] = useState<"not-started" | "in-progress" | "verified">("not-started");
  const [selectedProvider, setSelectedProvider] = useState("sumsub");
  const [isVerifying, setIsVerifying] = useState(false);

  const levels: KycLevel[] = [
    {
      level: 1,
      name: "Unverified",
      requirements: [],
      verified: true,
      limit: "$0 / $10k daily",
    },
    {
      level: 2,
      name: "Level 1 (Light)",
      requirements: ["Email", "Phone"],
      verified: kycStatus === "verified" && currentLevel >= 2,
      limit: "$500 / unlimited",
    },
    {
      level: 3,
      name: "Level 2 (Full)",
      requirements: ["ID Verification", "Address Proof", "Selfie"],
      verified: kycStatus === "verified" && currentLevel >= 3,
      limit: "$100k / unlimited",
    },
    {
      level: 4,
      name: "Level 3 (Institutional)",
      requirements: ["Company Verification", "Bank Account", "Tax ID"],
      verified: kycStatus === "verified" && currentLevel >= 4,
      limit: "Unlimited",
    },
  ];

  const handleVerify = () => {
    setIsVerifying(true);
    setTimeout(() => {
      setKycStatus("verified");
      setCurrentLevel(3);
      setIsVerifying(false);
    }, 2000);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} /> KYC/AML Verification
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Status Badge */}
        <div className={clsx(
          "p-4 rounded-lg border flex items-start gap-3",
          kycStatus === "verified"
            ? "bg-green-600/10 border-green-600"
            : kycStatus === "in-progress"
            ? "bg-yellow-600/10 border-yellow-600"
            : "bg-blue-600/10 border-blue-600"
        )}>
          {kycStatus === "verified" ? (
            <>
              <CheckCircle className="text-green-400 flex-shrink-0 mt-0.5" size={16} />
              <div>
                <div className="font-semibold text-green-400">✓ Verified</div>
                <div className="text-xs text-gray-400">Level {currentLevel} KYC approved</div>
              </div>
            </>
          ) : kycStatus === "in-progress" ? (
            <>
              <AlertCircle className="text-yellow-400 flex-shrink-0 mt-0.5 animate-pulse" size={16} />
              <div>
                <div className="font-semibold text-yellow-400">Verification in Progress</div>
                <div className="text-xs text-gray-400">Check back in 10-15 minutes</div>
              </div>
            </>
          ) : (
            <>
              <Fingerprint className="text-blue-400 flex-shrink-0 mt-0.5" size={16} />
              <div>
                <div className="font-semibold text-blue-400">Not Verified</div>
                <div className="text-xs text-gray-400">You are currently at general trading limits</div>
              </div>
            </>
          )}
        </div>

        {/* KYC Level Cards */}
        <div>
          <h3 className="font-semibold mb-3">Verification Tiers</h3>
          <div className="space-y-3">
            {levels.map((level) => (
              <div
                key={level.level}
                className={clsx(
                  "p-4 rounded-lg border-2 transition",
                  level.verified
                    ? "border-green-600 bg-green-600/5"
                    : currentLevel >= level.level
                    ? "border-blue-600 bg-blue-600/5"
                    : "border-[#2a2a35] bg-[#15151b]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <h4 className="font-semibold flex items-center gap-2">
                      {level.name}
                      {level.verified && <CheckCircle size={14} className="text-green-400" />}
                    </h4>
                    <p className="text-xs text-gray-400 mt-1">Limit: {level.limit}</p>
                  </div>
                </div>

                {level.requirements.length > 0 && (
                  <div className="text-xs space-y-1 mt-2">
                    {level.requirements.map((req, i) => (
                      <div key={i} className="text-gray-400 flex items-center gap-2">
                        <span className="w-1 h-1 bg-gray-400 rounded-full" />
                        {req}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Provider Selection */}
        {kycStatus !== "verified" && (
          <>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 flex items-center gap-2">
                <FileText size={16} /> Choose Verification Provider
              </h3>
              <div className="space-y-2">
                {[
                  { id: "sumsub", name: "Sumsub (Recommended)", desc: "Global KYC/AML, 5-10 min" },
                  { id: "persona", name: "Persona", desc: "US/EU KYC, 2-5 min" },
                  { id: "fractal", name: "Fractal", desc: "Crypto native, 1-3 min" },
                ].map((provider) => (
                  <label key={provider.id} className="flex items-center gap-3 p-2 rounded cursor-pointer hover:bg-[#2a2a35]">
                    <input
                      type="radio"
                      checked={selectedProvider === provider.id}
                      onChange={() => setSelectedProvider(provider.id)}
                      className="w-4 h-4"
                    />
                    <div>
                      <div className="text-sm font-medium">{provider.name}</div>
                      <div className="text-xs text-gray-400">{provider.desc}</div>
                    </div>
                  </label>
                ))}
              </div>
            </div>

            <button
              onClick={handleVerify}
              disabled={isVerifying}
              className={clsx(
                "w-full py-3 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2",
                isVerifying
                  ? "bg-gray-600 cursor-not-allowed"
                  : "bg-blue-600 hover:bg-blue-700"
              )}
            >
              {isVerifying ? (
                <>
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  Verifying...
                </>
              ) : (
                <>
                  <Shield size={16} /> Start Verification
                </>
              )}
            </button>
          </>
        )}

        {/* Verified Info */}
        {kycStatus === "verified" && (
          <div className="bg-green-600/10 border border-green-600 rounded-lg p-4">
            <h3 className="font-semibold text-green-400 mb-2">✓ Verification Complete</h3>
            <div className="space-y-1 text-xs text-gray-400">
              <div>Status: Level {currentLevel} Approved</div>
              <div>Verified on: Feb 28, 2026</div>
              <div>Expires: Feb 28, 2027</div>
            </div>
          </div>
        )}
      </div>

      <button className="w-full bg-[#15151b] hover:bg-[#1a1a20] py-2 rounded-lg font-semibold text-sm transition border border-[#2a2a35]">
        Learn More About KYC Requirements
      </button>
    </div>
  );
}
