import React, { useMemo, useState } from 'react';
import { AlertCircle, CheckCircle, ChevronDown, Clock, Settings } from 'lucide-react';
import clsx from 'clsx';
import {
  DESKTOP_READINESS_STATUS,
  FEATURE_STATUSES,
  summarizeFeatureModes,
  type FeatureMode,
} from '@/data/projectStatus';

interface ChangelogEntry {
  version: string;
  date: string;
  title: string;
  verified: string[];
  gaps: string[];
  evidence: string[];
}

const CHANGELOG: ChangelogEntry[] = [
  {
    version: '0.1.0',
    date: 'May 5, 2026',
    title: 'Guarded desktop readiness pass',
    verified: [
      'Tauri v2 shell is present under apps/x3-desktop/src-tauri.',
      'Local system metrics and IPFS panels have backend IPC commands.',
      'Feature modes are now shown from the project readiness registry snapshot.',
    ],
    gaps: [
      'Signed auto-update channel is not implemented.',
      'Backend app registry still returns an empty list.',
      'Some app launch targets need service health checks before opening.',
    ],
    evidence: [
      'apps/x3-desktop/src-tauri/tauri.conf.json',
      'FEATURE_REGISTRY.toml',
      'TESTNET_FEATURE_FLAGS.toml',
    ],
  },
  {
    version: '0.0.9',
    date: 'Earlier workspace state',
    title: 'Static desktop registry fallback',
    verified: [
      'Frontend registry can populate the desktop when backend registry is empty.',
      'Internal panels cover wallet, DEX, explorer-style pages, telemetry, and service dashboards.',
    ],
    gaps: [
      'Static descriptions had optimistic claims that were not tied to current feature modes.',
      'CI scripts did not run real build/test gates.',
    ],
    evidence: [
      'apps/x3-desktop/src/services/applicationService.ts',
      'apps/x3-desktop/package.json',
    ],
  },
];

const MODE_LABELS: Record<FeatureMode, string> = {
  LIVE_TESTNET: 'Live testnet',
  GUARDED_TESTNET: 'Guarded testnet',
  SIM_TESTNET: 'Simulation testnet',
  DISABLED_BLOCKED: 'Disabled/blocked',
};

const MODE_CLASSES: Record<FeatureMode, string> = {
  LIVE_TESTNET: 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300',
  GUARDED_TESTNET: 'border-amber-500/40 bg-amber-500/10 text-amber-300',
  SIM_TESTNET: 'border-cyan-500/40 bg-cyan-500/10 text-cyan-300',
  DISABLED_BLOCKED: 'border-red-500/40 bg-red-500/10 text-red-300',
};

const DesktopUpdatesPanel: React.FC = () => {
  const [expandedVersion, setExpandedVersion] = useState<string | null>(CHANGELOG[0]?.version ?? null);
  const modeCounts = useMemo(() => summarizeFeatureModes(), []);

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Settings size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">Desktop Readiness</h1>
        </div>
        <div className="flex items-center gap-2 text-xs font-mono text-gray-500">
          v{DESKTOP_READINESS_STATUS.currentVersion}
        </div>
      </div>

      <div className="border border-amber-500/40 bg-amber-500/10 m-5 rounded-lg p-5">
        <div className="flex items-start gap-4">
          <div className="bg-amber-500/20 rounded-lg p-3 flex-shrink-0">
            <AlertCircle size={20} className="text-amber-300" />
          </div>
          <div className="flex-1">
            <h3 className="font-bold text-white mb-1 flex items-center gap-2">
              Guarded testnet desktop
              <span className="text-xs bg-amber-500/20 text-amber-200 px-2 py-0.5 rounded">NOT AUTO-UPDATABLE</span>
            </h3>
            <p className="text-sm text-gray-300 mb-3">{DESKTOP_READINESS_STATUS.summary}</p>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
              {(Object.keys(modeCounts) as FeatureMode[]).map((mode) => (
                <div key={mode} className={clsx('rounded border px-3 py-2', MODE_CLASSES[mode])}>
                  <div className="font-mono text-base">{modeCounts[mode]}</div>
                  <div>{MODE_LABELS[mode]}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      <div className="px-5 pb-4">
        <h2 className="text-sm font-semibold text-gray-400 mb-3">Feature Registry Snapshot</h2>
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-3">
          {FEATURE_STATUSES.map((feature) => (
            <div key={feature.id} className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4">
              <div className="flex items-start justify-between gap-3 mb-3">
                <div>
                  <h3 className="font-semibold text-sm text-white">{feature.name}</h3>
                  <p className="text-xs text-gray-500">Tauri app: {feature.tauriApp}</p>
                </div>
                <span className={clsx('text-[10px] px-2 py-1 rounded border whitespace-nowrap', MODE_CLASSES[feature.mode])}>
                  {MODE_LABELS[feature.mode]}
                </span>
              </div>
              <p className="text-xs text-gray-400 mb-3">{feature.risk}</p>
              <div className="text-[11px] text-gray-500 font-mono">{feature.proofReport}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="px-5 py-4">
        <h2 className="text-sm font-semibold text-gray-400 mb-3">Readiness History</h2>
        
        <div className="space-y-3">
          {CHANGELOG.map((entry) => (
            <div
              key={entry.version}
              className="bg-[#111111] border border-[#1a1a1a] rounded-lg overflow-hidden hover:border-[#2a2a2a] transition-colors"
            >
              <button
                onClick={() => setExpandedVersion(expandedVersion === entry.version ? null : entry.version)}
                className="w-full flex items-center justify-between p-4 hover:bg-[#0f0f14] transition-colors"
              >
                <div className="flex items-center gap-3">
                  {entry.version === DESKTOP_READINESS_STATUS.currentVersion ? (
                    <div className="bg-blue-500/20 rounded-lg p-2">
                      <CheckCircle size={16} className="text-blue-400" />
                    </div>
                  ) : (
                    <div className="bg-[#0a0a0f] rounded-lg p-2">
                      <Clock size={16} className="text-gray-500" />
                    </div>
                  )}
                  <div className="text-left">
                    <div className="font-semibold text-white flex items-center gap-2">
                      v{entry.version}
                      {entry.version === DESKTOP_READINESS_STATUS.currentVersion && (
                        <span className="text-xs bg-blue-500/40 text-blue-300 px-2 py-0.5 rounded">Current</span>
                      )}
                    </div>
                    <div className="text-xs text-gray-500">{entry.date} - {entry.title}</div>
                  </div>
                </div>
                <ChevronDown
                  size={16}
                  className={clsx(
                    'text-gray-500 transition-transform',
                    expandedVersion === entry.version && 'rotate-180'
                  )}
                />
              </button>

              {/* Expanded Content */}
              {expandedVersion === entry.version && (
                <div className="border-t border-[#1a1a1a] bg-[#0a0a0f] p-4 space-y-4">
                  {entry.verified.length > 0 && (
                    <div>
                      <h4 className="text-xs font-semibold text-emerald-400 mb-2">Verified</h4>
                      <ul className="space-y-1 text-xs text-gray-300">
                        {entry.verified.map((item, idx) => (
                          <li key={idx} className="flex gap-2">
                            <span className="text-emerald-400">-</span> {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {entry.gaps.length > 0 && (
                    <div>
                      <h4 className="text-xs font-semibold text-amber-400 mb-2">Gaps</h4>
                      <ul className="space-y-1 text-xs text-gray-300">
                        {entry.gaps.map((item, idx) => (
                          <li key={idx} className="flex gap-2">
                            <span className="text-amber-400">-</span> {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {entry.evidence.length > 0 && (
                    <div>
                      <h4 className="text-xs font-semibold text-blue-400 mb-2">Evidence</h4>
                      <ul className="space-y-1 text-xs text-gray-300">
                        {entry.evidence.map((item, idx) => (
                          <li key={idx} className="flex gap-2">
                            <span className="text-blue-400">-</span> {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="mt-auto px-5 py-4 border-t border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-lg p-4 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Release channel</span>
            <span className="text-xs text-amber-300">manual / unsigned</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Source of truth</span>
            <span className="text-xs text-gray-500">{DESKTOP_READINESS_STATUS.sourceOfTruth}</span>
          </div>
          <div className="text-xs text-gray-600 pt-2 border-t border-[#1a1a1a]">
            Last reviewed: {DESKTOP_READINESS_STATUS.lastReviewed}
          </div>
        </div>
      </div>
    </div>
  );
};

export default DesktopUpdatesPanel;

