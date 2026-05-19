import React, { useState } from "react";
import { Users, Shield, CheckCircle, AlertCircle, Trash2, Plus } from "lucide-react";
import clsx from "clsx";

interface Guardian {
  id: string;
  address: string;
  name: string;
  status: "active" | "pending" | "revoked";
  joinedDate: string;
  recoveryVotes?: number;
}

interface RecoveryRequest {
  id: string;
  initiatedDate: string;
  type: "address-change" | "key-rotation";
  newAddress?: string;
  votesNeeded: number;
  votesReceived: number;
  expiry: string;
  status: "pending" | "approved" | "rejected";
}

const MOCK_GUARDIANS: Guardian[] = [
  {
    id: "1",
    address: "0x1234...5678",
    name: "Alice (Friend)",
    status: "active",
    joinedDate: "Dec 15, 2024",
    recoveryVotes: 1,
  },
  {
    id: "2",
    address: "0x8765...4321",
    name: "Bob (Lawyer)",
    status: "active",
    joinedDate: "Dec 20, 2024",
    recoveryVotes: 0,
  },
  {
    id: "3",
    address: "0xabcd...efgh",
    name: "Charlie (Advisor)",
    status: "pending",
    joinedDate: "-",
    recoveryVotes: 0,
  },
];

const MOCK_RECOVERY: RecoveryRequest[] = [
  {
    id: "1",
    initiatedDate: "Feb 28, 2025",
    type: "address-change",
    newAddress: "0xnewadd...1234",
    votesNeeded: 2,
    votesReceived: 1,
    expiry: "Mar 7, 2025",
    status: "pending",
  },
];

export default function SocialRecoveryPanel() {
  const [guardians, setGuardians] = useState<Guardian[]>(MOCK_GUARDIANS);
  const [showAddGuardian, setShowAddGuardian] = useState(false);
  const [newGuardianAddress, setNewGuardianAddress] = useState("");
  const [newGuardianName, setNewGuardianName] = useState("");
  const [selectedGuardian, setSelectedGuardian] = useState<Guardian | null>(null);
  const [recoveryRequests, setRecoveryRequests] = useState<RecoveryRequest[]>(MOCK_RECOVERY);

  const activeGuardians = guardians.filter((g) => g.status === "active").length;
  const requiredGuardians = Math.ceil(guardians.length / 2) + 1;

  const handleAddGuardian = () => {
    if (newGuardianAddress && newGuardianName) {
      const newGuardian: Guardian = {
        id: String(guardians.length + 1),
        address: newGuardianAddress,
        name: newGuardianName,
        status: "pending",
        joinedDate: "-",
        recoveryVotes: 0,
      };
      setGuardians([...guardians, newGuardian]);
      setNewGuardianAddress("");
      setNewGuardianName("");
      setShowAddGuardian(false);
    }
  };

  const handleRemoveGuardian = (id: string) => {
    setGuardians(guardians.filter((g) => g.id !== id));
    setSelectedGuardian(null);
  };

  const handleVoteApproval = (recoveryId: string) => {
    setRecoveryRequests(
      recoveryRequests.map((r) => {
        if (r.id === recoveryId && r.votesReceived < r.votesNeeded) {
          return {
            ...r,
            votesReceived: r.votesReceived + 1,
            status: r.votesReceived + 1 >= r.votesNeeded ? "approved" : "pending",
          };
        }
        return r;
      })
    );
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} className="text-purple-400" /> Social Recovery
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Recovery Status Card */}
        <div className="bg-gradient-to-r from-purple-600/20 to-blue-600/20 border border-purple-600 rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">Recovery Setup</h3>
          <div className="grid grid-cols-2 gap-3">
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Active Guardians</div>
              <div className="text-xl font-bold text-purple-400">{activeGuardians}</div>
            </div>
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Votes Needed</div>
              <div className="text-xl font-bold text-blue-400">{requiredGuardians}</div>
            </div>
          </div>
          <div className="mt-3 p-2 bg-purple-600/30 border border-purple-600 rounded text-xs text-purple-300">
            ✓ Multi-sig wallet recovery enabled. Guardians can restore access if you lose your keys.
          </div>
        </div>

        {/* Guardians Section */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-semibold text-sm flex items-center gap-2">
              <Users size={16} /> Guardians ({guardians.length})
            </h3>
            <button
              onClick={() => setShowAddGuardian(true)}
              className="flex items-center gap-1 text-xs bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded transition"
            >
              <Plus size={12} /> Add
            </button>
          </div>

          <div className="space-y-2">
            {guardians.map((guardian) => (
              <button
                key={guardian.id}
                onClick={() => setSelectedGuardian(guardian)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedGuardian?.id === guardian.id
                    ? "border-purple-600 bg-purple-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold">{guardian.name}</div>
                    <div className="text-xs font-mono text-gray-400">{guardian.address}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded border",
                      guardian.status === "active"
                        ? "bg-green-600/30 border-green-600 text-green-400"
                        : "bg-yellow-600/30 border-yellow-600 text-yellow-400"
                    )}
                  >
                    {guardian.status}
                  </span>
                </div>
                <div className="text-xs text-gray-400">Joined: {guardian.joinedDate}</div>
              </button>
            ))}
          </div>

          {/* Add Guardian Modal */}
          {showAddGuardian && (
            <div className="mt-3 bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <div>
                <label className="text-xs text-gray-400 block mb-2">Guardian Name</label>
                <input
                  type="text"
                  placeholder="e.g., Alice (Friend)"
                  value={newGuardianName}
                  onChange={(e) => setNewGuardianName(e.target.value)}
                  className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
                />
              </div>
              <div>
                <label className="text-xs text-gray-400 block mb-2">Wallet Address</label>
                <input
                  type="text"
                  placeholder="0x..."
                  value={newGuardianAddress}
                  onChange={(e) => setNewGuardianAddress(e.target.value)}
                  className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
                />
              </div>
              <div className="flex gap-2">
                <button
                  onClick={handleAddGuardian}
                  className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition"
                >
                  Add Guardian
                </button>
                <button
                  onClick={() => setShowAddGuardian(false)}
                  className="flex-1 bg-[#2a2a35] hover:bg-[#3a3a45] py-2 rounded-lg font-semibold text-sm transition"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Selected Guardian Details */}
        {selectedGuardian && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold text-sm">Guardian Details</h3>
              <button
                onClick={() => handleRemoveGuardian(selectedGuardian.id)}
                className="text-red-400 hover:text-red-300 transition"
              >
                <Trash2 size={16} />
              </button>
            </div>

            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Address</span>
                <span className="font-mono text-xs">{selectedGuardian.address}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span
                  className={selectedGuardian.status === "active" ? "text-green-400" : "text-yellow-400"}
                >
                  {selectedGuardian.status}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Joined</span>
                <span>{selectedGuardian.joinedDate}</span>
              </div>
            </div>

            {selectedGuardian.status === "pending" && (
              <div className="mt-3 p-2 bg-yellow-600/20 border border-yellow-600 rounded text-xs text-yellow-300">
                Waiting for guardian to accept the invite...
              </div>
            )}
          </div>
        )}

        {/* Active Recovery Requests */}
        {recoveryRequests.length > 0 && (
          <div>
            <h3 className="font-semibold mb-3 text-sm flex items-center gap-2">
              <AlertCircle size={16} className="text-orange-400" /> Active Recovery Requests
            </h3>

            <div className="space-y-2">
              {recoveryRequests.map((request) => (
                <button
                  key={request.id}
                  className="w-full text-left p-3 rounded-lg border-2 border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <div className="text-sm font-semibold">Address Change Request</div>
                      <div className="text-xs text-gray-400">Initiated: {request.initiatedDate}</div>
                    </div>
                    <span
                      className={clsx(
                        "text-xs px-2 py-1 rounded border",
                        request.status === "approved"
                          ? "bg-green-600/30 border-green-600 text-green-400"
                          : "bg-orange-600/30 border-orange-600 text-orange-400"
                      )}
                    >
                      {request.status}
                    </span>
                  </div>

                  <div className="mb-2 text-xs text-gray-400">
                    New Address: <span className="font-mono">{request.newAddress}</span>
                  </div>

                  <div className="mb-3">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-xs text-gray-400">
                        Votes: {request.votesReceived}/{request.votesNeeded}
                      </span>
                      <span className="text-xs text-gray-400">Expires: {request.expiry}</span>
                    </div>
                    <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-orange-600 to-yellow-600"
                        style={{ width: `${(request.votesReceived / request.votesNeeded) * 100}%` }}
                      />
                    </div>
                  </div>

                  {request.status === "pending" && (
                    <button
                      onClick={() => handleVoteApproval(request.id)}
                      className="w-full bg-green-600 hover:bg-green-700 py-2 rounded-lg text-xs font-semibold transition"
                    >
                      Vote to Approve
                    </button>
                  )}
                  {request.status === "approved" && (
                    <div className="text-xs text-green-400 flex items-center gap-1">
                      <CheckCircle size={12} /> Recovery approved by guardians
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Recovery Info */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">How It Works</h3>
          <ul className="space-y-2 text-xs text-gray-400">
            <li className="flex gap-2">
              <span className="text-blue-400 flex-shrink-0">1.</span>
              <span>Designate trusted guardians who can help recover your wallet</span>
            </li>
            <li className="flex gap-2">
              <span className="text-blue-400 flex-shrink-0">2.</span>
              <span>If you lose access, guardians vote to approve recovery</span>
            </li>
            <li className="flex gap-2">
              <span className="text-blue-400 flex-shrink-0">3.</span>
              <span>Majority consensus restores access (no single point of failure)</span>
            </li>
          </ul>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Social recovery adds resiliency without sacrificing decentralization.
      </div>
    </div>
  );
}
