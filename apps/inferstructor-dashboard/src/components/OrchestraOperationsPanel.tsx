import {
  Activity,
  AlertTriangle,
  ArrowRight,
  CheckCircle,
  Clock,
  FileText,
  Loader2,
  RefreshCw,
  Shield,
  Zap,
} from 'lucide-react';
import type {
  AdminJob,
  AggregatedMetrics,
  ApprovalCase,
  BenchmarkReport,
  EvidenceBundle,
  OrchestraIntent,
  TpsBenchmarkStatus,
  VoteTally,
  VoteWindow,
} from '../api';
import { useMemo, useState } from 'react';
import {
  benchmarkJobsByStatus,
  benchmarkIntentCounts,
  buildOrchestraIncidents,
  formatTimestamp,
  humanize,
} from './orchestra-incidents';

interface OrchestraOperationsPanelProps {
  services: Record<string, string>;
  aggregated?: AggregatedMetrics['aggregated'] | null;
  intents: OrchestraIntent[];
  approvalCases: ApprovalCase[];
  voteWindows: VoteWindow[];
  evidenceBundles: EvidenceBundle[];
  benchmarkStatus: TpsBenchmarkStatus | null;
  benchmarkReports: BenchmarkReport[];
  jobs: AdminJob[];
  loading: boolean;
  error: string | null;
  onRefresh: () => void | Promise<void>;
  onCloseVoteWindow: (windowId: string) => Promise<void>;
  onImportVoteTally: (windowId: string) => Promise<VoteTally>;
}

function normalize(value: string | null | undefined): string {
  return (value || '').trim().toLowerCase();
}

export function OrchestraOperationsPanel({
  services,
  aggregated,
  intents,
  approvalCases,
  voteWindows,
  evidenceBundles,
  benchmarkStatus,
  benchmarkReports,
  jobs,
  loading,
  error,
  onRefresh,
  onCloseVoteWindow,
  onImportVoteTally,
}: OrchestraOperationsPanelProps) {
  const [selectedApprovalCaseId, setSelectedApprovalCaseId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionLoadingId, setActionLoadingId] = useState<string | null>(null);
  const [importedTallies, setImportedTallies] = useState<Record<string, VoteTally>>({});

  const benchmarkIntents = intents.filter(intent => normalize(intent.kind) === 'benchmarking');
  const benchmarkIntentStatus = benchmarkIntentCounts(benchmarkIntents);
  const benchmarkJobStatus = benchmarkJobsByStatus(jobs);
  const openApprovalCases = approvalCases.filter(approvalCase => normalize(approvalCase.status) === 'open');
  const activeVoteWindows = voteWindows.filter(window => {
    const status = normalize(window.status);
    return status === 'open' || status === 'scheduled';
  });
  const publishedReports = benchmarkReports.filter(report => normalize(report.profile) === 'provider_onboarding');
  const sortedEvidence = [...evidenceBundles].sort(
    (left, right) => new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime(),
  );
  const selectedApprovalCase = useMemo(() => {
    if (!selectedApprovalCaseId) {
      return openApprovalCases[0] || approvalCases[0] || null;
    }

    return approvalCases.find(approvalCase => approvalCase.case_id === selectedApprovalCaseId) || null;
  }, [approvalCases, openApprovalCases, selectedApprovalCaseId]);

  const evidenceForSelectedApproval = useMemo(() => {
    if (!selectedApprovalCase) {
      return sortedEvidence;
    }

    const relatedVoteWindowIds = voteWindows
      .filter(window => window.approval_case_id === selectedApprovalCase.case_id)
      .map(window => window.window_id);

    const relatedIntentId = selectedApprovalCase.intent_id;

    return sortedEvidence.filter(bundle => {
      return bundle.approval_case_id === selectedApprovalCase.case_id
        || bundle.intent_id === relatedIntentId
        || (bundle.vote_window_id ? relatedVoteWindowIds.includes(bundle.vote_window_id) : false);
    });
  }, [selectedApprovalCase, sortedEvidence, voteWindows]);

  const incidents = buildOrchestraIncidents({
    services,
    aggregated,
    approvalCases,
    voteWindows,
    intents,
    evidenceBundles,
    benchmarkStatus,
    jobs,
  });

  const handleCloseVoteWindow = async (windowId: string) => {
    try {
      setActionError(null);
      setActionLoadingId(`close:${windowId}`);
      await onCloseVoteWindow(windowId);
      await onRefresh();
    } catch {
      setActionError(`Failed to close vote window ${windowId}.`);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleImportVoteTally = async (windowId: string) => {
    try {
      setActionError(null);
      setActionLoadingId(`import:${windowId}`);
      const tally = await onImportVoteTally(windowId);
      setImportedTallies(current => ({ ...current, [windowId]: tally }));
      await onRefresh();
    } catch {
      setActionError(`Failed to import tally for vote window ${windowId}.`);
    } finally {
      setActionLoadingId(null);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-bold text-white">Operator Workflow</h2>
          <p className="text-sm text-gray-400">
            Approval backlog, incident pressure, benchmark publication, and evidence lineage.
          </p>
        </div>
        <button
          onClick={() => void onRefresh()}
          className="inline-flex items-center gap-2 rounded-lg border border-gray-700 bg-gray-800/60 px-3 py-2 text-sm text-gray-200 transition-colors hover:bg-gray-700/60"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          Refresh
        </button>
      </div>

      {error && (
        <div className="rounded-lg border border-yellow-700/50 bg-yellow-900/20 px-4 py-3 text-sm text-yellow-200">
          {error}
        </div>
      )}

      {actionError && (
        <div className="rounded-lg border border-red-700/50 bg-red-900/20 px-4 py-3 text-sm text-red-200">
          {actionError}
        </div>
      )}

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
          <Shield className="mb-2 h-5 w-5 text-blue-400" />
          <p className="font-mono text-2xl font-bold text-white">{openApprovalCases.length}</p>
          <p className="text-xs text-gray-400">Open approval cases</p>
        </div>
        <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
          <AlertTriangle className="mb-2 h-5 w-5 text-orange-400" />
          <p className="font-mono text-2xl font-bold text-white">{incidents.length}</p>
          <p className="text-xs text-gray-400">Active incidents</p>
        </div>
        <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
          <Zap className="mb-2 h-5 w-5 text-yellow-400" />
          <p className="font-mono text-2xl font-bold text-white">{publishedReports.length}</p>
          <p className="text-xs text-gray-400">Published onboarding reports</p>
        </div>
        <div className="rounded-lg border border-gray-700 bg-gray-800/50 p-4">
          <FileText className="mb-2 h-5 w-5 text-green-400" />
          <p className="font-mono text-2xl font-bold text-white">{evidenceBundles.length}</p>
          <p className="text-xs text-gray-400">Evidence bundles</p>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-2">
        <div className="card">
          <div className="mb-4 flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-orange-400" />
            <h3 className="text-lg font-bold text-white">Incident Feed</h3>
          </div>
          <div className="space-y-3">
            {incidents.length === 0 && (
              <div className="rounded-lg border border-green-700/30 bg-green-900/10 px-4 py-3 text-sm text-green-300">
                No active operator incidents. Gateway, sidecar, and orchestra workflow checks are clear.
              </div>
            )}
            {incidents.map(incident => (
              <div
                key={incident.id}
                className="rounded-lg border border-gray-700 bg-gray-800/40 p-4"
              >
                <div className="mb-1 flex items-center justify-between gap-3">
                  <span className="text-sm font-semibold text-white">{incident.title}</span>
                  <span
                    className={`rounded px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${
                      incident.severity === 'critical'
                        ? 'bg-red-500/20 text-red-300'
                        : incident.severity === 'warning'
                          ? 'bg-yellow-500/20 text-yellow-300'
                          : 'bg-blue-500/20 text-blue-300'
                    }`}
                  >
                    {incident.severity}
                  </span>
                </div>
                <p className="mb-1 text-xs text-gray-400">Source: {humanize(incident.source)}</p>
                <p className="text-sm text-gray-200">{incident.detail}</p>
              </div>
            ))}
          </div>
        </div>

        <div className="card">
          <div className="mb-4 flex items-center gap-2">
            <Shield className="h-5 w-5 text-blue-400" />
            <h3 className="text-lg font-bold text-white">Approval Pipeline</h3>
          </div>
          <div className="grid gap-6 lg:grid-cols-2">
            <div>
              <div className="mb-3 flex items-center justify-between">
                <span className="text-sm font-semibold text-gray-200">Approval cases</span>
                <span className="text-xs text-gray-500">{openApprovalCases.length} open</span>
              </div>
              <div className="space-y-3">
                {openApprovalCases.slice(0, 5).map(approvalCase => (
                  <button
                    key={approvalCase.case_id}
                    type="button"
                    onClick={() => setSelectedApprovalCaseId(approvalCase.case_id)}
                    className={`block w-full rounded-lg border p-3 text-left transition-colors ${
                      selectedApprovalCase?.case_id === approvalCase.case_id
                        ? 'border-blue-500/60 bg-blue-950/20'
                        : 'border-gray-700 bg-gray-800/40 hover:bg-gray-800/60'
                    }`}
                  >
                    <div className="mb-1 flex items-center justify-between gap-3">
                      <span className="text-sm font-semibold text-white">{approvalCase.summary}</span>
                      <span className="rounded bg-blue-500/20 px-2 py-0.5 text-[10px] font-semibold text-blue-300">
                        {humanize(approvalCase.review_kind)}
                      </span>
                    </div>
                    <p className="text-xs text-gray-400">Requested by {approvalCase.requested_by}</p>
                    <p className="mt-1 text-xs text-gray-500">Opened {formatTimestamp(approvalCase.created_at)}</p>
                  </button>
                ))}
                {openApprovalCases.length === 0 && (
                  <p className="text-sm text-gray-500">No open approval cases.</p>
                )}
              </div>
            </div>
            <div>
              <div className="mb-3 flex items-center justify-between">
                <span className="text-sm font-semibold text-gray-200">Vote windows</span>
                <span className="text-xs text-gray-500">{activeVoteWindows.length} active</span>
              </div>
              <div className="space-y-3">
                {activeVoteWindows.slice(0, 5).map(window => {
                  const tally = window.tally || {};
                  const importedTally = importedTallies[window.window_id];
                  const isClosing = actionLoadingId === `close:${window.window_id}`;
                  const isImporting = actionLoadingId === `import:${window.window_id}`;
                  const isOverdue = (normalize(window.status) === 'open' || normalize(window.status) === 'scheduled')
                    && window.closes_at_unix < Math.floor(Date.now() / 1000);
                  return (
                    <div key={window.window_id} className="rounded-lg border border-gray-700 bg-gray-800/40 p-3">
                      <div className="mb-1 flex items-center justify-between gap-3">
                        <span className="text-sm font-semibold text-white">{window.title}</span>
                        <span className="rounded bg-purple-500/20 px-2 py-0.5 text-[10px] font-semibold text-purple-300">
                          {humanize(window.status)}
                        </span>
                      </div>
                      <p className="text-xs text-gray-400">Closes {formatTimestamp(window.closes_at_unix)}</p>
                      <p className="mt-1 text-xs text-gray-500">
                        Tally {tally.approvals || 0}/{tally.rejections || 0}/{tally.abstentions || 0}
                      </p>
                      {importedTally && (
                        <p className="mt-1 text-xs text-green-300">
                          Imported {importedTally.approvals}/{importedTally.rejections}/{importedTally.abstentions}
                        </p>
                      )}
                      <div className="mt-3 flex flex-wrap gap-2">
                        <button
                          type="button"
                          onClick={() => void handleImportVoteTally(window.window_id)}
                          disabled={isImporting}
                          className="inline-flex items-center gap-2 rounded-md border border-gray-600 bg-gray-900/60 px-2.5 py-1.5 text-xs text-gray-200 hover:bg-gray-800 disabled:cursor-wait disabled:opacity-60"
                        >
                          {isImporting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <ArrowRight className="h-3.5 w-3.5" />}
                          Import tally
                        </button>
                        {isOverdue && (
                          <button
                            type="button"
                            onClick={() => void handleCloseVoteWindow(window.window_id)}
                            disabled={isClosing}
                            className="inline-flex items-center gap-2 rounded-md border border-red-700/50 bg-red-900/30 px-2.5 py-1.5 text-xs text-red-200 hover:bg-red-900/50 disabled:cursor-wait disabled:opacity-60"
                          >
                            {isClosing ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <AlertTriangle className="h-3.5 w-3.5" />}
                            Close overdue window
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
                {activeVoteWindows.length === 0 && (
                  <p className="text-sm text-gray-500">No scheduled or open vote windows.</p>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-2">
        <div className="card">
          <div className="mb-4 flex items-center gap-2">
            <Zap className="h-5 w-5 text-yellow-400" />
            <h3 className="text-lg font-bold text-white">Benchmark Health</h3>
          </div>

          <div className="mb-4 rounded-lg border border-gray-700 bg-gray-800/40 p-4">
            <div className="mb-2 flex items-center justify-between text-sm">
              <span className="text-gray-300">Coverage progress</span>
              <span className="font-mono text-white">{benchmarkStatus?.progress_pct ?? 0}%</span>
            </div>
            <div className="h-2 rounded-full bg-gray-700">
              <div
                className="h-2 rounded-full bg-yellow-500 transition-all"
                style={{ width: `${Math.min(benchmarkStatus?.progress_pct ?? 0, 100)}%` }}
              />
            </div>
            <div className="mt-2 flex items-center justify-between text-xs text-gray-500">
              <span>{benchmarkStatus?.measured ?? 0} measured</span>
              <span>{benchmarkStatus?.total ?? 0} total targets</span>
              <span>Updated {benchmarkStatus?.last_updated ? formatTimestamp(benchmarkStatus.last_updated) : '—'}</span>
            </div>
          </div>

          <div className="mb-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">{benchmarkIntents.length}</p>
              <p className="text-xs text-gray-400">Benchmark intents</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">{benchmarkIntentStatus.completed || 0}</p>
              <p className="text-xs text-gray-400">Completed intents</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">{benchmarkJobStatus.running || 0}</p>
              <p className="text-xs text-gray-400">Running jobs</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">{benchmarkJobStatus.failed || 0}</p>
              <p className="text-xs text-gray-400">Failed jobs</p>
            </div>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-semibold text-gray-200">Recent onboarding reports</span>
              <span className="text-xs text-gray-500">{publishedReports.length} published</span>
            </div>
            {publishedReports.slice(0, 5).map(report => (
              <div key={report.report_id} className="rounded-lg border border-gray-700 bg-gray-800/40 p-3">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-sm font-semibold text-white">{report.chain_name}</span>
                  <span className="rounded bg-green-500/20 px-2 py-0.5 text-[10px] font-semibold text-green-300">
                    {humanize(report.profile)}
                  </span>
                </div>
                <p className="mt-1 text-xs text-gray-400">
                  {report.recommendation} via {report.signer}
                </p>
                <p className="mt-1 text-xs text-gray-500">
                  Generated {formatTimestamp(report.generated_at_unix)} · {report.workload_profile?.total_transactions || 0} txns
                </p>
              </div>
            ))}
            {publishedReports.length === 0 && (
              <p className="text-sm text-gray-500">No published onboarding benchmark reports yet.</p>
            )}
          </div>
        </div>

        <div className="card">
          <div className="mb-4 flex items-center gap-2">
            <FileText className="h-5 w-5 text-green-400" />
            <h3 className="text-lg font-bold text-white">Evidence Status</h3>
          </div>

          {selectedApprovalCase && (
            <div className="mb-4 rounded-lg border border-blue-700/30 bg-blue-950/10 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-semibold text-white">Evidence drill-down</p>
                  <p className="text-xs text-gray-400">{selectedApprovalCase.summary}</p>
                </div>
                <span className="rounded bg-blue-500/20 px-2 py-0.5 text-[10px] font-semibold text-blue-300">
                  {humanize(selectedApprovalCase.status)}
                </span>
              </div>
            </div>
          )}

          <div className="mb-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">{evidenceBundles.length}</p>
              <p className="text-xs text-gray-400">Bundles</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">
                {evidenceBundles.filter(bundle => bundle.intent_id).length}
              </p>
              <p className="text-xs text-gray-400">Intent-linked</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">
                {evidenceBundles.filter(bundle => bundle.approval_case_id).length}
              </p>
              <p className="text-xs text-gray-400">Approval-linked</p>
            </div>
            <div className="rounded-lg border border-gray-700 bg-gray-800/30 p-3">
              <p className="font-mono text-xl font-bold text-white">
                {evidenceBundles.filter(bundle => bundle.vote_window_id).length}
              </p>
              <p className="text-xs text-gray-400">Vote-linked</p>
            </div>
          </div>

          <div className="space-y-3">
            {evidenceForSelectedApproval.slice(0, 6).map(bundle => (
              <div key={bundle.bundle_id} className="rounded-lg border border-gray-700 bg-gray-800/40 p-3">
                <div className="mb-1 flex items-center justify-between gap-3">
                  <span className="text-sm font-semibold text-white">
                    {humanize(typeof bundle.summary?.action === 'string' ? bundle.summary.action : 'evidence_bundle')}
                  </span>
                  <span className="text-xs text-gray-500">{formatTimestamp(bundle.updated_at)}</span>
                </div>
                <p className="truncate text-xs text-gray-400">{bundle.artifact_uri}</p>
                <div className="mt-2 flex flex-wrap gap-2 text-[10px] text-gray-500">
                  {bundle.intent_id && <span className="rounded bg-gray-700 px-2 py-0.5">intent {bundle.intent_id}</span>}
                  {bundle.approval_case_id && <span className="rounded bg-gray-700 px-2 py-0.5">approval {bundle.approval_case_id}</span>}
                  {bundle.vote_window_id && <span className="rounded bg-gray-700 px-2 py-0.5">vote {bundle.vote_window_id}</span>}
                </div>
              </div>
            ))}
            {evidenceForSelectedApproval.length === 0 && (
              <p className="text-sm text-gray-500">
                {selectedApprovalCase
                  ? 'No evidence bundles linked to the selected approval case yet.'
                  : 'No evidence bundles are recorded yet.'}
              </p>
            )}
          </div>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <div className="rounded-lg border border-gray-700 bg-gray-800/40 p-4">
          <div className="mb-2 flex items-center gap-2 text-gray-200">
            <Activity className="h-4 w-4 text-blue-400" />
            <span className="text-sm font-semibold">Gateway pulse</span>
          </div>
          <p className="text-xs text-gray-400">
            Bridge failures: {aggregated?.bridge_failed || 0} · RPC errors: {aggregated?.rpc_errors || 0}
          </p>
        </div>
        <div className="rounded-lg border border-gray-700 bg-gray-800/40 p-4">
          <div className="mb-2 flex items-center gap-2 text-gray-200">
            <Clock className="h-4 w-4 text-yellow-400" />
            <span className="text-sm font-semibold">Sidecar execution</span>
          </div>
          <p className="text-xs text-gray-400">
            Running benchmark jobs: {benchmarkJobStatus.running || 0} · Failed: {benchmarkJobStatus.failed || 0}
          </p>
        </div>
        <div className="rounded-lg border border-gray-700 bg-gray-800/40 p-4">
          <div className="mb-2 flex items-center gap-2 text-gray-200">
            <CheckCircle className="h-4 w-4 text-green-400" />
            <span className="text-sm font-semibold">Workflow lineage</span>
          </div>
          <p className="text-xs text-gray-400">
            {intents.filter(intent => intent.requires_approval).length} approval-bound intents · {evidenceBundles.length} evidence records
          </p>
        </div>
      </div>
    </div>
  );
}