import React, { useState } from 'react';
import { CheckCircle, Plus, Search, ChevronDown, Lock, Unlock } from 'lucide-react';

interface Validator {
  id: string;
  name: string;
  status: 'approved' | 'pending' | 'suspended';
  uptime: number;
  signedBlocks: number;
  lastVoteTime: string;
  chain: string;
}

interface ValidatorControlsProps {
  onClose?: () => void;
}

export const ValidatorControls: React.FC<ValidatorControlsProps> = () => {
  const [validators, setValidators] = useState<Validator[]>([
    {
      id: 'val-001',
      name: 'MainNet Validator 1',
      status: 'approved',
      uptime: 99.87,
      signedBlocks: 45230,
      lastVoteTime: '2 seconds ago',
      chain: 'Ethereum',
    },
    {
      id: 'val-002',
      name: 'Solana Validator 1',
      status: 'approved',
      uptime: 98.92,
      signedBlocks: 38921,
      lastVoteTime: '5 seconds ago',
      chain: 'Solana',
    },
    {
      id: 'val-003',
      name: 'Pending Validator',
      status: 'pending',
      uptime: 95.2,
      signedBlocks: 12000,
      lastVoteTime: '1 minute ago',
      chain: 'Ethereum',
    },
  ]);

  const [searchTerm, setSearchTerm] = useState('');
  const [showActionMenu, setShowActionMenu] = useState<string | null>(null);

  const filteredValidators = validators.filter(
    (v) =>
      v.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      v.id.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleApprove = (validator: Validator) => {
    setValidators(
      validators.map((v) =>
        v.id === validator.id ? { ...v, status: 'approved' } : v
      )
    );
    setShowActionMenu(null);
  };

  const handleSuspend = (validator: Validator) => {
    setValidators(
      validators.map((v) =>
        v.id === validator.id ? { ...v, status: 'suspended' } : v
      )
    );
    setShowActionMenu(null);
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'approved':
        return (
          <div className="flex items-center gap-1 px-3 py-1 bg-green-900 text-green-300 rounded-full text-xs">
            <CheckCircle className="w-3 h-3" />
            Approved
          </div>
        );
      case 'pending':
        return (
          <div className="flex items-center gap-1 px-3 py-1 bg-yellow-900 text-yellow-300 rounded-full text-xs">
            <ChevronDown className="w-3 h-3" />
            Pending
          </div>
        );
      case 'suspended':
        return (
          <div className="flex items-center gap-1 px-3 py-1 bg-red-900 text-red-300 rounded-full text-xs">
            <Lock className="w-3 h-3" />
            Suspended
          </div>
        );
      default:
        return null;
    }
  };

  return (
    <div className="px-6">
      <div className="max-w-6xl">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-white mb-2">Validator Controls</h1>
          <p className="text-gray-400">Manage validator approvals, suspensions, and chain onboarding</p>
        </div>

        {/* Search and Add */}
        <div className="mb-6 flex gap-4">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-3 w-5 h-5 text-gray-500" />
            <input
              type="text"
              placeholder="Search by validator name or ID..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-10 pr-4 py-2 bg-[#1a1a2e] border border-[#2a2a35] rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
            />
          </div>
          <button className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg flex items-center gap-2 transition-colors">
            <Plus className="w-4 h-4" />
            Add Validator
          </button>
        </div>

        {/* Validators Table */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
          <table className="w-full">
            <thead>
              <tr className="border-b border-[#2a2a35]">
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Validator</th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Chain</th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Uptime</th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Signed Blocks</th>
                <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Status</th>
                <th className="px-6 py-4 text-right text-sm font-semibold text-gray-300">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredValidators.map((validator) => (
                <tr key={validator.id} className="border-b border-[#2a2a35] hover:bg-[#2a2a35] transition-colors">
                  <td className="px-6 py-4">
                    <div>
                      <p className="text-white font-medium">{validator.name}</p>
                      <p className="text-gray-500 text-sm">{validator.id}</p>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-gray-300">{validator.chain}</td>
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2">
                      <div className="w-20 bg-gray-700 rounded-full h-2">
                        <div
                          className="bg-green-500 h-2 rounded-full"
                          style={{ width: `${validator.uptime}%` }}
                        />
                      </div>
                      <span className="text-gray-300 text-sm">{validator.uptime.toFixed(2)}%</span>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-gray-300">{validator.signedBlocks.toLocaleString()}</td>
                  <td className="px-6 py-4">{getStatusBadge(validator.status)}</td>
                  <td className="px-6 py-4 text-right">
                    <div className="relative inline-block">
                      <button
                        onClick={() => setShowActionMenu(showActionMenu === validator.id ? null : validator.id)}
                        className="p-2 hover:bg-gray-700 rounded-lg text-gray-400 hover:text-white transition-colors"
                      >
                        <ChevronDown className="w-4 h-4" />
                      </button>
                      {showActionMenu === validator.id && (
                        <div className="absolute right-0 mt-1 bg-[#2a2a35] border border-[#3a3a45] rounded-lg py-1 z-10">
                          {validator.status !== 'approved' && (
                            <button
                              onClick={() => handleApprove(validator)}
                              className="w-full px-4 py-2 text-left text-sm text-green-400 hover:bg-gray-700 flex items-center gap-2 transition-colors"
                            >
                              <CheckCircle className="w-3 h-3" />
                              Approve
                            </button>
                          )}
                          {validator.status !== 'suspended' && (
                            <button
                              onClick={() => handleSuspend(validator)}
                              className="w-full px-4 py-2 text-left text-sm text-red-400 hover:bg-gray-700 flex items-center gap-2 transition-colors"
                            >
                              <Lock className="w-3 h-3" />
                              Suspend
                            </button>
                          )}
                          {validator.status === 'suspended' && (
                            <button
                              onClick={() => handleApprove(validator)}
                              className="w-full px-4 py-2 text-left text-sm text-blue-400 hover:bg-gray-700 flex items-center gap-2 transition-colors"
                            >
                              <Unlock className="w-3 h-3" />
                              Unlock
                            </button>
                          )}
                        </div>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mt-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <p className="text-gray-400 text-sm mb-2">Total Validators</p>
            <p className="text-2xl font-bold text-white">{validators.length}</p>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <p className="text-gray-400 text-sm mb-2">Approved</p>
            <p className="text-2xl font-bold text-green-400">{validators.filter(v => v.status === 'approved').length}</p>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <p className="text-gray-400 text-sm mb-2">Pending</p>
            <p className="text-2xl font-bold text-yellow-400">{validators.filter(v => v.status === 'pending').length}</p>
          </div>
        </div>
      </div>
    </div>
  );
};
