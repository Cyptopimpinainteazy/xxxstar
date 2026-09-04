import type {
  AdminJob,
  AggregatedMetrics,
  ApprovalCase,
  EvidenceBundle,
  OrchestraIntent,
  TpsBenchmarkStatus,
  VoteWindow,
} from '../api';

export interface OrchestraIncident {
  id: string;
  severity: 'critical' | 'warning' | 'info';
  source: 'gateway' | 'sidecar' | 'orchestra-control-plane' | 'workflow';
  title: string;
  detail: string;
}

export interface BuildOrchestraIncidentInput {
  services: Record<string, string>;
  aggregated?: AggregatedMetrics['aggregated'] | null;
  approvalCases: ApprovalCase[];
  voteWindows: VoteWindow[];
  intents: OrchestraIntent[];
  evidenceBundles: EvidenceBundle[];
  benchmarkStatus?: TpsBenchmarkStatus | null;
  jobs: AdminJob[];
  nowUnix?: number;
}

const severityRank: Record<OrchestraIncident['severity'], number> = {
  critical: 0,
  warning: 1,
  info: 2,
};

function normalize(value: string | null | undefined): string {
  return (value || '').trim().toLowerCase();
}

function matchServiceStatus(services: Record<string, string>, needle: string): string | null {
  const entry = Object.entries(services).find(([name]) => normalize(name).includes(needle));
  return entry ? entry[1] : null;
}

export function benchmarkJobsByStatus(jobs: AdminJob[]): Record<string, number> {
  return jobs.reduce<Record<string, number>>((counts, job) => {
    if (!normalize(job.command).includes('bench')) {
      return counts;
    }
    counts[job.status] = (counts[job.status] || 0) + 1;
    return counts;
  }, {});
}

export function benchmarkIntentCounts(intents: OrchestraIntent[]): Record<string, number> {
  return intents.reduce<Record<string, number>>((counts, intent) => {
    if (normalize(intent.kind) !== 'benchmarking') {
      return counts;
    }
    const status = normalize(intent.status);
    counts[status] = (counts[status] || 0) + 1;
    return counts;
  }, {});
}

export function humanize(value: string | null | undefined): string {
  if (!value) {
    return 'unknown';
  }

  return value
    .split(/[_-]/g)
    .filter(Boolean)
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function formatTimestamp(value: string | number | null | undefined): string {
  if (value == null) {
    return '—';
  }

  const date = typeof value === 'number' ? new Date(value * 1000) : new Date(value);

  if (Number.isNaN(date.getTime())) {
    return '—';
  }

  return date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function buildOrchestraIncidents({
  services,
  aggregated,
  approvalCases,
  voteWindows,
  intents,
  evidenceBundles,
  benchmarkStatus,
  jobs,
  nowUnix = Math.floor(Date.now() / 1000),
}: BuildOrchestraIncidentInput): OrchestraIncident[] {
  const incidents: OrchestraIncident[] = [];
  const expectedServices = [
    { key: 'gateway', label: 'Gateway', source: 'gateway' as const },
    { key: 'sidecar', label: 'Sidecar', source: 'sidecar' as const },
    { key: 'orchestra', label: 'Orchestra Control Plane', source: 'orchestra-control-plane' as const },
  ];

  expectedServices.forEach(service => {
    const status = normalize(matchServiceStatus(services, service.key));
    if (status && status !== 'up') {
      incidents.push({
        id: `service-${service.key}`,
        severity: 'critical',
        source: service.source,
        title: `${service.label} degraded`,
        detail: `${service.label} reports ${status}. Operator routing is not healthy.`,
      });
    }
  });

  if ((aggregated?.bridge_failed || 0) > 0) {
    incidents.push({
      id: 'gateway-bridge-failures',
      severity: 'warning',
      source: 'gateway',
      title: 'Bridge delivery failures detected',
      detail: `${aggregated?.bridge_failed || 0} bridge requests failed in the current metrics window.`,
    });
  }

  if ((aggregated?.rpc_errors || 0) > 0) {
    incidents.push({
      id: 'gateway-rpc-errors',
      severity: 'warning',
      source: 'gateway',
      title: 'RPC proxy errors accumulating',
      detail: `${aggregated?.rpc_errors || 0} proxy errors reported by the gateway metrics feed.`,
    });
  }

  const overdueVoteWindows = voteWindows.filter(window => {
    const status = normalize(window.status);
    return (status === 'open' || status === 'scheduled') && window.closes_at_unix < nowUnix;
  });
  if (overdueVoteWindows.length > 0) {
    incidents.push({
      id: 'workflow-overdue-votes',
      severity: 'critical',
      source: 'workflow',
      title: 'Vote windows passed their close time',
      detail: `${overdueVoteWindows.length} vote window${overdueVoteWindows.length === 1 ? '' : 's'} need closure or tally import.`,
    });
  }

  const staleApprovals = approvalCases.filter(approvalCase => {
    if (normalize(approvalCase.status) !== 'open') {
      return false;
    }
    const openedAt = new Date(approvalCase.created_at).getTime();
    if (Number.isNaN(openedAt)) {
      return false;
    }
    return openedAt / 1000 < nowUnix - 3600;
  });
  if (staleApprovals.length > 0) {
    incidents.push({
      id: 'workflow-stale-approvals',
      severity: 'warning',
      source: 'workflow',
      title: 'Approval queue is aging',
      detail: `${staleApprovals.length} approval case${staleApprovals.length === 1 ? '' : 's'} have been open for more than 1 hour.`,
    });
  }

  const terminalIntents = intents.filter(intent => {
    const status = normalize(intent.status);
    return status === 'dispatched' || status === 'completed';
  });
  const missingEvidenceCount = terminalIntents.filter(intent => {
    return !evidenceBundles.some(bundle => bundle.intent_id === intent.intent_id);
  }).length;
  if (missingEvidenceCount > 0) {
    incidents.push({
      id: 'workflow-missing-evidence',
      severity: 'warning',
      source: 'orchestra-control-plane',
      title: 'Completed workflow without evidence',
      detail: `${missingEvidenceCount} dispatched or completed intent${missingEvidenceCount === 1 ? '' : 's'} do not have a linked evidence bundle.`,
    });
  }

  const benchmarkJobCounts = benchmarkJobsByStatus(jobs);
  if ((benchmarkJobCounts.failed || 0) > 0) {
    incidents.push({
      id: 'sidecar-benchmark-failures',
      severity: 'warning',
      source: 'sidecar',
      title: 'Benchmark jobs failed',
      detail: `${benchmarkJobCounts.failed} recent benchmark job${benchmarkJobCounts.failed === 1 ? '' : 's'} ended in failure.`,
    });
  }

  if (benchmarkStatus && benchmarkStatus.progress_pct === 0 && terminalIntents.length > 0) {
    incidents.push({
      id: 'workflow-benchmark-stalled',
      severity: 'info',
      source: 'sidecar',
      title: 'Benchmark coverage has not advanced',
      detail: 'The benchmark status endpoint still reports 0% progress despite workflow activity.',
    });
  }

  return incidents.sort((left, right) => severityRank[left.severity] - severityRank[right.severity]);
}