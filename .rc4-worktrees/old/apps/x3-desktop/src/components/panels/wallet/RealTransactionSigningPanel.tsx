import React, { useState } from "react";
import { Signature, CheckCircle, Clock, AlertTriangle, Copy, Eye, Send } from "lucide-react";
import clsx from "clsx";

interface SigningRequest {
  id: string;
  title: string;
  description: string;
  txData: string;
  from: string;
  to: string;
  value: number;
  gas: number;
  gasPrice: number;
  nonce: number;
  timestamp: string;
  status: "pending" | "signed" | "failed" | "submitted";
}

interface SignedTransaction {
  id: string;
  signature: string;
  txHash: string;
  timestamp: string;
  status: "broadcast" | "confirmed" | "failed";
  confirmations: number;
}

const MOCK_REQUESTS: SigningRequest[] = [
  {
    id: "1",
    title: "Swap X3 → USDC",
    description: "DEX swap on Uniswap V3",
    txData: "0xa9059cbb000000000000000000000000...",
    from: "0x742d35Cc6634C0532925a3b844Bc9e7595f...7e6f",
    to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    value: 1000,
    gas: 150000,
    gasPrice: 45.5,
    nonce: 42,
    timestamp: "2024-10-05T14:32:00Z",
    status: "pending",
  },
  {
    id: "2",
    title: "Token Approval",
    description: "Approve USDC for DEX",
    txData: "0x095ea7b3000000000000000000000000...",
    from: "0x742d35Cc6634C0532925a3b844Bc9e7595f...7e6f",
    to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    value: 0,
    gas: 45000,
    gasPrice: 45.5,
    nonce: 41,
    timestamp: "2024-10-05T14:20:00Z",
    status: "signed",
  },
];

const MOCK_SIGNED: SignedTransaction[] = [
  {
    id: "1",
    signature: "0x3045022100a1b2c3d4...f6e7d8c9",
    txHash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    timestamp: "2024-10-05T14:15:00Z",
    status: "confirmed",
    confirmations: 12,
  },
  {
    id: "2",
    signature: "0x3044022050e4f3a2...b9a8c7d6",
    txHash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    timestamp: "2024-10-05T14:00:00Z",
    status: "confirmed",
    confirmations: 85,
  },
];

export default function RealTransactionSigningPanel() {
  const [requests, setRequests] = useState<SigningRequest[]>(MOCK_REQUESTS);
  const [signed, setSigned] = useState<SignedTransaction[]>(MOCK_SIGNED);
  const [selectedRequest, setSelectedRequest] = useState<SigningRequest | null>(MOCK_REQUESTS[0]);
  const [activeTab, setActiveTab] = useState<"pending" | "signed">("pending");
  const [showRawData, setShowRawData] = useState(false);

  const totalFee = selectedRequest ? selectedRequest.gas * (selectedRequest.gasPrice / 1000) : 0;

  const handleSign = (requestId: string) => {
    const updated = requests.map((r) =>
      r.id === requestId
        ? { ...r, status: "signed" as const }
        : r
    );
    setRequests(updated);

    const newSigned: SignedTransaction = {
      id: requestId,
      signature: "0x" + Array(132).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join(""),
      txHash: "0x" + Array(64).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join(""),
      timestamp: new Date().toISOString(),
      status: "broadcast",
      confirmations: 0,
    };
    setSigned([newSigned, ...signed]);
  };

  const handleReject = (requestId: string) => {
    setRequests(requests.map((r) =>
      r.id === requestId ? { ...r, status: "failed" as const } : r
    ));
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Signature size={20} className="text-blue-400" /> Transaction Signing
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Pending</div>
            <div className="text-lg font-bold text-yellow-400">{requests.filter(r => r.status === "pending").length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Signed</div>
            <div className="text-lg font-bold text-green-400">{requests.filter(r => r.status === "signed").length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Confirmed</div>
            <div className="text-lg font-bold text-cyan-400">{signed.filter(s => s.status === "confirmed").length}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["pending", "signed"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2",
                activeTab === tab
                  ? "border-cyan-600 text-cyan-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "pending" ? "Pending" : "Signed"}
            </button>
          ))}
        </div>

        {activeTab === "pending" && (
          <div className="space-y-3">
            {requests.filter(r => r.status === "pending" || r.status === "failed").map((request) => (
              <button
                key={request.id}
                onClick={() => setSelectedRequest(request)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedRequest?.id === request.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="font-semibold">{request.title}</div>
                  <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", request.status === "failed" ? "bg-red-600/20 text-red-400" : "bg-yellow-600/20 text-yellow-400")}>
                    {request.status.toUpperCase()}
                  </span>
                </div>
                <div className="text-xs text-gray-400 mb-2">{request.description}</div>
                <div className="flex justify-between text-xs text-gray-500">
                  <span>Nonce: {request.nonce}</span>
                  <span>Gas: {request.gas.toLocaleString()}</span>
                </div>
              </button>
            ))}
          </div>
        )}

        {activeTab === "signed" && (
          <div className="space-y-2">
            {signed.map((tx) => (
              <div key={tx.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div className="font-semibold text-sm">TX #{tx.id}</div>
                  <div className="flex items-center gap-2">
                    {tx.status === "confirmed" ? (
                      <CheckCircle size={14} className="text-green-400" />
                    ) : (
                      <Clock size={14} className="text-yellow-400 animate-spin" />
                    )}
                    <span className="text-xs font-bold">{tx.confirmations} confirmations</span>
                  </div>
                </div>

                <div className="text-xs text-gray-400 font-mono">{tx.txHash}</div>
                <div className="text-xs text-gray-500">{tx.timestamp}</div>
              </div>
            ))}
          </div>
        )}

        {/* Request Details */}
        {selectedRequest && activeTab === "pending" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="font-semibold">{selectedRequest.title}</h3>
              <button
                onClick={() => setShowRawData(!showRawData)}
                className="text-cyan-400 hover:text-cyan-300 flex items-center gap-1 text-xs"
              >
                <Eye size={12} /> {showRawData ? "Hide" : "Show"} Raw
              </button>
            </div>

            {showRawData ? (
              <div className="bg-[#0a0a0f] p-3 rounded border border-[#2a2a35] font-mono text-xs text-gray-400 max-h-32 overflow-y-auto break-all">
                {selectedRequest.txData}
              </div>
            ) : (
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">From</span>
                  <span className="font-mono text-xs">{selectedRequest.from.slice(0, 10)}...{selectedRequest.from.slice(-8)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">To</span>
                  <span className="font-mono text-xs">{selectedRequest.to.slice(0, 10)}...{selectedRequest.to.slice(-8)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Value</span>
                  <span className="font-semibold">{selectedRequest.value} X3</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Gas</span>
                  <span className="font-semibold">{selectedRequest.gas.toLocaleString()}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Gas Price</span>
                  <span className="font-semibold">{selectedRequest.gasPrice.toFixed(2)} Gwei</span>
                </div>
                <div className="border-t border-[#2a2a35] pt-2 flex justify-between font-bold">
                  <span>Total Fee</span>
                  <span className="text-yellow-400">${totalFee.toFixed(2)}</span>
                </div>
              </div>
            )}

            <div className="flex gap-2 pt-2">
              <button
                onClick={() => handleSign(selectedRequest.id)}
                className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
              >
                <Signature size={14} /> Sign & Approve
              </button>
              <button
                onClick={() => handleReject(selectedRequest.id)}
                className="flex-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 py-2 rounded-lg font-semibold text-sm transition"
              >
                Reject
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Sign transactions with your local keystore or hardware wallet.
      </div>
    </div>
  );
}
