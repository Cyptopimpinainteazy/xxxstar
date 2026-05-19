import React, { useState, useEffect, useCallback, useMemo } from "react";
import {
  Search,
  Blocks,
  ArrowRight,
  RefreshCw,
  Activity,
  CheckCircle,
  XCircle,
  Clock,
  Hash,
  Users,
  Wifi,
  ChevronRight,
  Loader2,
  AlertTriangle,
} from "lucide-react";
import {
  useNetworkStats,
  useRecentBlocks,
  useRecentExtrinsics,
  useNewHeads,
  useAuthorities,
} from "@/hooks/useSubstrate";

/* ------------------------------------------------------------------ */
/*  VM type badge                                                      */
/* ------------------------------------------------------------------ */
function vmBadge(section: string) {
  if (section.toLowerCase().includes("evm"))
    return (
      <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-blue-500/20 text-blue-400 border border-blue-500/30">
        EVM
      </span>
    );
  if (section.toLowerCase().includes("svm"))
    return (
      <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-violet-500/20 text-violet-400 border border-violet-500/30">
        SVM
      </span>
    );
  if (
    section.toLowerCase().includes("atlaskernel") ||
    section.toLowerCase().includes("comit")
  )
    return (
      <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-orange-500/20 text-orange-400 border border-orange-500/30">
        Comit
      </span>
    );
  return (
    <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-slate-500/20 text-slate-400 border border-slate-500/30">
      Native
    </span>
  );
}

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */
function shortHash(hash: string | undefined, chars = 8) {
  if (!hash) return "—";
  if (hash.length <= chars * 2 + 3) return hash;
  return `${hash.slice(0, chars)}…${hash.slice(-chars)}`;
}

function timeAgo(ts: number | string | undefined) {
  if (!ts) return "—";
  const diff = Math.max(
    0,
    Math.floor((Date.now() - (typeof ts === "string" ? parseInt(ts, 10) : ts)) / 1000)
  );
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  return `${Math.floor(diff / 3600)}h ago`;
}

/* ------------------------------------------------------------------ */
/*  Spinner & Error                                                    */
/* ------------------------------------------------------------------ */
function Spinner({ className = "" }: { className?: string }) {
  return <Loader2 className={`animate-spin ${className}`} />;
}

function ErrorBanner({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="flex items-center gap-3 bg-red-500/10 border border-red-500/30 rounded-xl px-4 py-3 text-red-400 text-sm">
      <AlertTriangle className="w-5 h-5 shrink-0" />
      <span className="flex-1">{message}</span>
      {onRetry && (
        <button onClick={onRetry} className="text-red-300 hover:text-white transition text-xs underline">
          Retry
        </button>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Tabs                                                               */
/* ------------------------------------------------------------------ */
type Tab = "overview" | "blocks" | "transactions" | "accounts";

const tabs: { key: Tab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "blocks", label: "Blocks" },
  { key: "transactions", label: "Transactions" },
  { key: "accounts", label: "Accounts" },
];

/* ================================================================== */
/*  BlockExplorerPanel                                                 */
/* ================================================================== */
export default function BlockExplorerPanel() {
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [search, setSearch] = useState("");

  /* ---------- hooks ---------- */
  const {
    data: networkStats,
    error: statsError,
    isLoading: statsLoading,
    mutate: mutateStats,
  } = useNetworkStats();

  const {
    data: blocks,
    error: blocksError,
    isLoading: blocksLoading,
    mutate: mutateBlocks,
  } = useRecentBlocks(20);

  const {
    data: extrinsics,
    error: extrinsicsError,
    isLoading: extrinsicsLoading,
    mutate: mutateExtrinsics,
  } = useRecentExtrinsics(30);

  const {
    data: authorities,
    error: authError,
    isLoading: authLoading,
  } = useAuthorities();

  const { data: latestHead } = useNewHeads();

  /* auto-refresh on new head */
  useEffect(() => {
    if (latestHead) {
      mutateStats();
      mutateBlocks();
      mutateExtrinsics();
    }
  }, [latestHead, mutateStats, mutateBlocks, mutateExtrinsics]);

  /* ---------- derived ---------- */
  const blockHeight = networkStats?.blockNumber ?? "—";
  const peerCount = networkStats?.peerCount ?? "—";
  const syncStatus =
    networkStats?.isSyncing === false
      ? "Synced"
      : networkStats?.isSyncing === true
      ? "Syncing…"
      : "—";
  const validatorCount = authorities?.length ?? "—";

  const anyError = statsError || blocksError || extrinsicsError || authError;
  const isInitialLoading = statsLoading && blocksLoading;

  /* ---------- search ---------- */
  const handleSearch = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      // In a real implementation this would navigate to block / tx / account detail
      console.log("Search:", search);
    },
    [search]
  );

  /* ---------- filtered extrinsics for search (basic) ---------- */
  const displayedExtrinsics = useMemo(() => {
    if (!extrinsics) return [];
    if (!search.trim()) return extrinsics;
    const q = search.toLowerCase();
    return extrinsics.filter(
      (ext) =>
        ext.hash?.toLowerCase().includes(q) ||
        ext.signer?.toLowerCase().includes(q) ||
        `${ext.section}.${ext.method}`.toLowerCase().includes(q)
    );
  }, [extrinsics, search]);

  /* ================================================================ */
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0f0a00] to-black text-white">
      {/* ---- Header ---- */}
      <div className="px-6 pt-6 pb-4 space-y-5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Blocks className="w-8 h-8 text-orange-400" />
            <div>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-amber-300 bg-clip-text text-transparent">
                Block Explorer
              </h1>
              <p className="text-slate-400 text-sm">X3 X3 Chain — Real-time</p>
            </div>
          </div>
          <button
            onClick={() => {
              mutateStats();
              mutateBlocks();
              mutateExtrinsics();
            }}
            className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 transition"
            title="Refresh"
          >
            <RefreshCw className="w-4 h-4 text-slate-400" />
          </button>
        </div>

        {/* ---- Search ---- */}
        <form onSubmit={handleSearch} className="relative">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search by block number, tx hash, or address…"
            className="w-full pl-10 pr-4 py-2.5 rounded-xl bg-slate-800/70 border border-slate-700/60 text-sm text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-orange-500/40 transition"
          />
        </form>

        {/* ---- Error banner ---- */}
        {anyError && (
          <ErrorBanner
            message={anyError.message ?? "Failed to load chain data"}
            onRetry={() => {
              mutateStats();
              mutateBlocks();
              mutateExtrinsics();
            }}
          />
        )}

        {/* ---- Live Stats ---- */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <StatCard
            icon={<Blocks className="w-5 h-5 text-orange-400" />}
            label="Block Height"
            value={statsLoading ? undefined : String(blockHeight)}
          />
          <StatCard
            icon={<Users className="w-5 h-5 text-amber-400" />}
            label="Validators"
            value={authLoading ? undefined : String(validatorCount)}
          />
          <StatCard
            icon={<Wifi className="w-5 h-5 text-cyan-400" />}
            label="Peers"
            value={statsLoading ? undefined : String(peerCount)}
          />
          <StatCard
            icon={<Activity className="w-5 h-5 text-green-400" />}
            label="Sync Status"
            value={statsLoading ? undefined : syncStatus}
            valueColor={
              syncStatus === "Synced"
                ? "text-green-400"
                : syncStatus === "Syncing…"
                ? "text-yellow-400"
                : undefined
            }
          />
        </div>

        {/* ---- Tab bar ---- */}
        <div className="flex gap-1 bg-slate-800/50 p-1 rounded-xl">
          {tabs.map((t) => (
            <button
              key={t.key}
              onClick={() => setActiveTab(t.key)}
              className={`flex-1 text-sm font-medium py-2 rounded-lg transition ${
                activeTab === t.key
                  ? "bg-orange-500/20 text-orange-300"
                  : "text-slate-400 hover:text-slate-200 hover:bg-slate-700/40"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>

      {/* ---- Content ---- */}
      <div className="px-6 pb-8 space-y-6">
        {isInitialLoading && (
          <div className="flex flex-col items-center justify-center py-20 gap-3 text-slate-400">
            <Spinner className="w-8 h-8 text-orange-400" />
            <span className="text-sm">Loading chain data…</span>
          </div>
        )}

        {!isInitialLoading && activeTab === "overview" && (
          <OverviewTab
            blocks={blocks ?? []}
            extrinsics={displayedExtrinsics.slice(0, 10)}
            blocksLoading={blocksLoading}
            extrinsicsLoading={extrinsicsLoading}
          />
        )}
        {!isInitialLoading && activeTab === "blocks" && (
          <BlocksTab blocks={blocks ?? []} loading={blocksLoading} />
        )}
        {!isInitialLoading && activeTab === "transactions" && (
          <TransactionsTab extrinsics={displayedExtrinsics} loading={extrinsicsLoading} />
        )}
        {!isInitialLoading && activeTab === "accounts" && <AccountsTab />}
      </div>
    </div>
  );
}

/* ================================================================== */
/*  StatCard                                                           */
/* ================================================================== */
function StatCard({
  icon,
  label,
  value,
  valueColor,
}: {
  icon: React.ReactNode;
  label: string;
  value?: string;
  valueColor?: string;
}) {
  return (
    <div className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 flex flex-col gap-1.5">
      <div className="flex items-center gap-2">
        {icon}
        <span className="text-xs text-slate-400">{label}</span>
      </div>
      {value ? (
        <span className={`text-lg font-bold ${valueColor ?? "text-white"}`}>{value}</span>
      ) : (
        <Spinner className="w-4 h-4 text-slate-500 mt-1" />
      )}
    </div>
  );
}

/* ================================================================== */
/*  OverviewTab                                                        */
/* ================================================================== */
function OverviewTab({
  blocks,
  extrinsics,
  blocksLoading,
  extrinsicsLoading,
}: {
  blocks: any[];
  extrinsics: any[];
  blocksLoading: boolean;
  extrinsicsLoading: boolean;
}) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      {/* Recent Blocks */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
        <div className="px-5 py-3.5 border-b border-slate-700/50 flex items-center justify-between">
          <h2 className="font-semibold flex items-center gap-2 text-sm">
            <Blocks className="w-4 h-4 text-orange-400" />
            Recent Blocks
          </h2>
        </div>
        {blocksLoading ? (
          <LoadingRows />
        ) : blocks.length === 0 ? (
          <EmptyState text="No blocks yet" />
        ) : (
          <div className="divide-y divide-slate-700/30">
            {blocks.slice(0, 8).map((b: any) => (
              <div
                key={b.number ?? b.hash}
                className="px-5 py-3 flex items-center justify-between hover:bg-slate-700/20 transition"
              >
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-lg bg-orange-500/10 flex items-center justify-center text-orange-400 font-bold text-xs">
                    {b.number != null ? `#${b.number}` : "—"}
                  </div>
                  <div>
                    <div className="text-sm font-medium text-slate-200">
                      {shortHash(b.hash)}
                    </div>
                    <div className="text-xs text-slate-500 flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      {timeAgo(b.timestamp)}
                      {b.extrinsicsCount != null && (
                        <span className="ml-2 text-slate-600">
                          {b.extrinsicsCount} extrinsics
                        </span>
                      )}
                    </div>
                  </div>
                </div>
                <ChevronRight className="w-4 h-4 text-slate-600" />
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Recent Extrinsics */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
        <div className="px-5 py-3.5 border-b border-slate-700/50 flex items-center justify-between">
          <h2 className="font-semibold flex items-center gap-2 text-sm">
            <Activity className="w-4 h-4 text-amber-400" />
            Recent Extrinsics
          </h2>
        </div>
        {extrinsicsLoading ? (
          <LoadingRows />
        ) : extrinsics.length === 0 ? (
          <EmptyState text="No extrinsics yet" />
        ) : (
          <div className="divide-y divide-slate-700/30">
            {extrinsics.slice(0, 8).map((ext: any, i: number) => (
              <div
                key={ext.hash ?? i}
                className="px-5 py-3 flex items-center justify-between hover:bg-slate-700/20 transition"
              >
                <div className="flex items-center gap-3">
                  <div className="shrink-0">
                    {ext.success !== false ? (
                      <CheckCircle className="w-4 h-4 text-green-400" />
                    ) : (
                      <XCircle className="w-4 h-4 text-red-400" />
                    )}
                  </div>
                  <div>
                    <div className="text-sm font-medium text-slate-200 flex items-center gap-2">
                      <span>
                        {ext.section}.{ext.method}
                      </span>
                      {vmBadge(ext.section ?? "")}
                    </div>
                    <div className="text-xs text-slate-500">
                      {shortHash(ext.hash)} · Block #{ext.blockNumber ?? "—"}
                    </div>
                  </div>
                </div>
                <ArrowRight className="w-4 h-4 text-slate-600" />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/* ================================================================== */
/*  BlocksTab                                                          */
/* ================================================================== */
function BlocksTab({ blocks, loading }: { blocks: any[]; loading: boolean }) {
  if (loading) return <LoadingRows count={8} />;
  if (blocks.length === 0) return <EmptyState text="No blocks available" />;
  return (
    <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-slate-400 border-b border-slate-700/50 text-xs">
              <th className="px-5 py-3 font-medium">Block</th>
              <th className="px-5 py-3 font-medium">Hash</th>
              <th className="px-5 py-3 font-medium">Parent Hash</th>
              <th className="px-5 py-3 font-medium">Extrinsics</th>
              <th className="px-5 py-3 font-medium">Time</th>
            </tr>
          </thead>
          <tbody>
            {blocks.map((b: any) => (
              <tr
                key={b.number ?? b.hash}
                className="border-b border-slate-700/30 hover:bg-slate-700/20 transition"
              >
                <td className="px-5 py-3 font-mono text-orange-400 font-medium">
                  #{b.number ?? "—"}
                </td>
                <td className="px-5 py-3 font-mono text-slate-300 text-xs">
                  {shortHash(b.hash, 10)}
                </td>
                <td className="px-5 py-3 font-mono text-slate-500 text-xs">
                  {shortHash(b.parentHash, 8)}
                </td>
                <td className="px-5 py-3 text-slate-300">{b.extrinsicsCount ?? "—"}</td>
                <td className="px-5 py-3 text-slate-500 text-xs flex items-center gap-1">
                  <Clock className="w-3 h-3" />
                  {timeAgo(b.timestamp)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  TransactionsTab                                                    */
/* ================================================================== */
function TransactionsTab({ extrinsics, loading }: { extrinsics: any[]; loading: boolean }) {
  if (loading) return <LoadingRows count={8} />;
  if (extrinsics.length === 0) return <EmptyState text="No transactions found" />;
  return (
    <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-slate-400 border-b border-slate-700/50 text-xs">
              <th className="px-5 py-3 font-medium">Hash</th>
              <th className="px-5 py-3 font-medium">Block</th>
              <th className="px-5 py-3 font-medium">Call</th>
              <th className="px-5 py-3 font-medium">VM</th>
              <th className="px-5 py-3 font-medium">Signer</th>
              <th className="px-5 py-3 font-medium">Result</th>
            </tr>
          </thead>
          <tbody>
            {extrinsics.map((ext: any, i: number) => (
              <tr
                key={ext.hash ?? i}
                className="border-b border-slate-700/30 hover:bg-slate-700/20 transition"
              >
                <td className="px-5 py-3 font-mono text-slate-300 text-xs">
                  {shortHash(ext.hash, 8)}
                </td>
                <td className="px-5 py-3 font-mono text-orange-400 text-xs">
                  #{ext.blockNumber ?? "—"}
                </td>
                <td className="px-5 py-3 text-slate-200 font-medium text-xs">
                  {ext.section}.{ext.method}
                </td>
                <td className="px-5 py-3">{vmBadge(ext.section ?? "")}</td>
                <td className="px-5 py-3 font-mono text-slate-400 text-xs">
                  {shortHash(ext.signer, 6) || "unsigned"}
                </td>
                <td className="px-5 py-3">
                  {ext.success !== false ? (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-green-500/20 text-green-400 border border-green-500/30">
                      <CheckCircle className="w-3 h-3" /> Success
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-red-500/20 text-red-400 border border-red-500/30">
                      <XCircle className="w-3 h-3" /> Failed
                    </span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  AccountsTab (placeholder)                                          */
/* ================================================================== */
function AccountsTab() {
  return (
    <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-10 text-center">
      <Users className="w-10 h-10 text-slate-600 mx-auto mb-3" />
      <h3 className="text-lg font-semibold text-slate-300 mb-1">Account Lookup</h3>
      <p className="text-sm text-slate-500">
        Search for an address above to view account details, balances, and transaction history.
      </p>
    </div>
  );
}

/* ================================================================== */
/*  Shared micro-components                                            */
/* ================================================================== */
function LoadingRows({ count = 5 }: { count?: number }) {
  return (
    <div className="p-5 space-y-3">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="h-10 rounded-lg bg-slate-700/30 animate-pulse" />
      ))}
    </div>
  );
}

function EmptyState({ text }: { text: string }) {
  return (
    <div className="py-12 text-center text-sm text-slate-500">
      <Hash className="w-6 h-6 mx-auto mb-2 text-slate-600" />
      {text}
    </div>
  );
}
