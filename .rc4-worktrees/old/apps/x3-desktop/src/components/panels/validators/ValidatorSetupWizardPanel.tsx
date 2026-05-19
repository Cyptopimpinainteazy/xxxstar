import React, { useState } from 'react';
import { ChevronRight, Zap, Lock, Radio, Play, CheckCircle } from 'lucide-react';
import clsx from 'clsx';

type Step = 'wallet' | 'stake' | 'rpc' | 'confirm' | 'done';

interface ValidatorSetup {
  walletAddress: string;
  stakeAmount: number;
  rpcEndpoint: string;
  gpuModel: string;
  isInstalled: boolean;
}

const ValidatorSetupWizardPanel: React.FC = () => {
  const [step, setStep] = useState<Step>('wallet');
  const [setup, setSetup] = useState<ValidatorSetup>({
    walletAddress: '',
    stakeAmount: 32,
    rpcEndpoint: 'https://rpc.x3chain.io',
    gpuModel: 'RTX 4090',
    isInstalled: false,
  });
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState(0);

  const steps: Step[] = ['wallet', 'stake', 'rpc', 'confirm', 'done'];
  const stepLabels: Record<Step, string> = {
    wallet: 'Connect Wallet',
    stake: 'Stake Amount',
    rpc: 'RPC Endpoint',
    confirm: 'Review & Launch',
    done: 'Installation Complete',
  };

  const handleNext = () => {
    const currentIdx = steps.indexOf(step);
    if (currentIdx < steps.length - 1) {
      setStep(steps[currentIdx + 1]);
    }
  };

  const handleInstall = () => {
    setInstalling(true);
    let progress = 0;
    const interval = setInterval(() => {
      progress += Math.random() * 20;
      if (progress > 100) {
        progress = 100;
        clearInterval(interval);
        setSetup({ ...setup, isInstalled: true });
        setInstalling(false);
        setStep('done');
      }
      setInstallProgress(Math.min(progress, 100));
    }, 500);
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Zap size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">Validator Setup Wizard</h1>
        </div>
        <div className="text-xs font-mono text-gray-500">{step.charAt(0).toUpperCase() + step.slice(1)}</div>
      </div>

      {/* Progress Steps */}
      <div className="px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center justify-between mb-3">
          {steps.map((s, idx) => (
            <div key={s} className="flex items-center">
              <div
                className={clsx(
                  'w-8 h-8 rounded-full flex items-center justify-center font-semibold text-xs transition-all',
                  steps.indexOf(step) >= idx
                    ? 'bg-blue-500 text-white'
                    : 'bg-[#111111] border border-[#1a1a1a] text-gray-500'
                )}
              >
                {steps.indexOf(step) > idx ? <CheckCircle size={16} /> : idx + 1}
              </div>
              {idx < steps.length - 1 && (
                <div
                  className={clsx(
                    'h-0.5 flex-1 mx-2 transition-all',
                    steps.indexOf(step) > idx ? 'bg-blue-500' : 'bg-[#1a1a1a]'
                  )}
                />
              )}
            </div>
          ))}
        </div>
        <div className="text-center text-sm font-semibold text-white">
          {stepLabels[step]}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 px-5 py-6 overflow-auto">
        {step === 'wallet' && (
          <div className="max-w-md mx-auto space-y-4">
            <h2 className="text-xl font-bold mb-4">Connect Your Wallet</h2>
            <p className="text-sm text-gray-400 mb-6">This wallet will receive your validator rewards and must have at least 32 X3 to stake.</p>
            <input
              type="text"
              value={setup.walletAddress}
              onChange={(e) => setSetup({ ...setup, walletAddress: e.target.value })}
              placeholder="0x..."
              className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 text-white text-sm outline-none focus:border-blue-500/40"
            />
            <div className="bg-blue-500/20 border border-blue-500/40 rounded-lg p-4 text-sm text-blue-400">
              <p>Ensure this is your personal wallet, not an exchange wallet. Rewards go here.</p>
            </div>
          </div>
        )}

        {step === 'stake' && (
          <div className="max-w-md mx-auto space-y-4">
            <h2 className="text-xl font-bold mb-4">Stake Amount</h2>
            <p className="text-sm text-gray-400 mb-6">Minimum 32 X3. More stake = higher earnings potential.</p>
            <div>
              <label className="block text-xs text-gray-500 mb-2">Amount (X3)</label>
              <div className="flex items-center gap-3">
                <input
                  type="number"
                  value={setup.stakeAmount}
                  onChange={(e) => setSetup({ ...setup, stakeAmount: Math.max(32, parseInt(e.target.value) || 32) })}
                  className="flex-1 bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 text-white text-sm outline-none focus:border-blue-500/40"
                />
                <span className="text-3xl font-bold text-blue-400">{setup.stakeAmount}</span>
              </div>
              <div className="mt-2 text-xs text-gray-500">
                Estimated annual reward: {(setup.stakeAmount * 0.12).toFixed(1)} X3 (12% APY)
              </div>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {[32, 64, 128].map((amt) => (
                <button
                  key={amt}
                  onClick={() => setSetup({ ...setup, stakeAmount: amt })}
                  className={clsx(
                    'py-2 px-3 rounded-lg text-xs font-semibold transition-all',
                    setup.stakeAmount === amt
                      ? 'bg-blue-500/30 border border-blue-500/60 text-blue-400'
                      : 'bg-[#111111] border border-[#1a1a1a] text-gray-400 hover:text-white'
                  )}
                >
                  {amt} X3
                </button>
              ))}
            </div>
          </div>
        )}

        {step === 'rpc' && (
          <div className="max-w-md mx-auto space-y-4">
            <h2 className="text-xl font-bold mb-4">RPC Endpoint</h2>
            <p className="text-sm text-gray-400 mb-6">URL for your validator to connect to the chain. Use default or custom.</p>
            <div>
              <label className="block text-xs text-gray-500 mb-2">RPC URL</label>
              <input
                type="text"
                value={setup.rpcEndpoint}
                onChange={(e) => setSetup({ ...setup, rpcEndpoint: e.target.value })}
                className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 text-white text-sm outline-none focus:border-blue-500/40 font-mono"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-2">GPU Model (for benchmarking)</label>
              <select
                value={setup.gpuModel}
                onChange={(e) => setSetup({ ...setup, gpuModel: e.target.value })}
                className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 text-white text-sm outline-none focus:border-blue-500/40"
              >
                <option>RTX 4090</option>
                <option>RTX 4080</option>
                <option>RTX 4070</option>
                <option>A100</option>
              </select>
            </div>
            <div className="bg-green-500/20 border border-green-500/40 rounded-lg p-4 text-sm text-green-400">
              ✓ RPC endpoint reachable
            </div>
          </div>
        )}

        {step === 'confirm' && (
          <div className="max-w-md mx-auto space-y-4">
            <h2 className="text-xl font-bold mb-4">Review Configuration</h2>
            <div className="space-y-3">
              <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a]">
                <div className="text-xs text-gray-500 mb-1">Wallet</div>
                <div className="font-mono text-sm text-white">{setup.walletAddress.slice(0, 16)}...</div>
              </div>
              <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a]">
                <div className="text-xs text-gray-500 mb-1">Stake Amount</div>
                <div className="text-lg font-bold text-blue-400">{setup.stakeAmount} X3</div>
              </div>
              <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a]">
                <div className="text-xs text-gray-500 mb-1">GPU / Hardware</div>
                <div className="font-semibold text-white">{setup.gpuModel}</div>
              </div>
              <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a]">
                <div className="text-xs text-gray-500 mb-1">RPC</div>
                <div className="font-mono text-xs text-white">{setup.rpcEndpoint}</div>
              </div>
            </div>
            <div className="bg-purple-500/20 border border-purple-500/40 rounded-lg p-4 text-sm text-purple-400 mt-4">
              Ready to launch your validator. Click below to install and start earning rewards!
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="max-w-md mx-auto text-center space-y-4">
            <div className="w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
              <CheckCircle size={32} className="text-green-400" />
            </div>
            <h2 className="text-2xl font-bold">Validator Live! 🎉</h2>
            <p className="text-gray-400">
              Your validator is now running and earning rewards. You'll receive blocks approximately every 10-15 seconds.
            </p>
            <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a] text-sm">
              <div className="text-xs text-gray-500 mb-1">Validator ID</div>
              <div className="font-mono text-blue-400">validator_0x123...abc</div>
            </div>
            <div className="bg-[#111111] rounded-lg p-4 border border-[#1a1a1a] text-sm">
              <div className="text-xs text-gray-500 mb-1">Est. Monthly Earnings</div>
              <div className="text-xl font-bold text-green-400">{(setup.stakeAmount * 0.12 / 12).toFixed(2)} X3</div>
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="border-t border-[#1a1a1a] px-5 py-4 flex gap-3 justify-end">
        {step !== 'done' && (
          <>
            <button
              onClick={() => {
                const currentIdx = steps.indexOf(step);
                if (currentIdx > 0) setStep(steps[currentIdx - 1]);
              }}
              disabled={step === 'wallet'}
              className="px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white disabled:text-gray-600 transition-colors"
            >
              Back
            </button>
            {step !== 'confirm' ? (
              <button
                onClick={handleNext}
                disabled={!setup.walletAddress && step === 'wallet'}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold disabled:from-gray-600 disabled:to-gray-600 transition-all"
              >
                Next <ChevronRight size={14} />
              </button>
            ) : (
              <button
                onClick={handleInstall}
                disabled={installing}
                className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-green-500 to-green-600 text-white font-semibold disabled:from-gray-600 disabled:to-gray-600 transition-all"
              >
                <Play size={14} /> Launch Validator
              </button>
            )}
          </>
        )}
      </div>

      {/* Install Progress */}
      {installing && (
        <div className="border-t border-[#1a1a1a] px-5 py-4 bg-[#111111]">
          <div className="flex justify-between text-xs mb-2">
            <span className="text-gray-400">Installing validator...</span>
            <span className="text-green-400">{Math.round(installProgress)}%</span>
          </div>
          <div className="w-full bg-[#0a0a0f] rounded-full h-2 border border-[#1a1a1a] overflow-hidden">
            <div
              className="bg-gradient-to-r from-green-500 to-green-600 h-full transition-all duration-300"
              style={{ width: `${installProgress}%` }}
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default ValidatorSetupWizardPanel;

