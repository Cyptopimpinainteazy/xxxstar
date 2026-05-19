import {
  Shield,
  Lock,
  Bug,
  AlertTriangle,
  CheckCircle,
  Mail,
  ExternalLink,
} from "lucide-react";

const securityMeasures = [
  {
    title: "Authorization System",
    description:
      "Multi-layered authorization with role-based access control, multi-sig requirements, and timelocked governance actions to prevent unauthorized state changes.",
    icon: Lock,
  },
  {
    title: "Atomic Execution",
    description:
      "All cross-chain transactions execute atomically — either all operations succeed or the entire batch is rolled back, ensuring no partial state corruption.",
    icon: Shield,
  },
  {
    title: "Prepare Root Verification",
    description:
      "State roots are prepared and verified against deterministic fixtures before finalization, preventing invalid state transitions from propagating.",
    icon: CheckCircle,
  },
  {
    title: "Third-Party Audits",
    description:
      "Regular comprehensive audits by top security firms including Trail of Bits and OpenZeppelin, with all findings publicly disclosed and remediated.",
    icon: ExternalLink,
  },
];

const bountyTiers = [
  { severity: "Critical", range: "$25,000 – $50,000", color: "text-red-400 bg-red-500/10 border-red-500/30" },
  { severity: "High", range: "$10,000 – $25,000", color: "text-orange-400 bg-orange-500/10 border-orange-500/30" },
  { severity: "Medium", range: "$2,500 – $10,000", color: "text-yellow-400 bg-yellow-500/10 border-yellow-500/30" },
  { severity: "Low", range: "$500 – $2,500", color: "text-green-400 bg-green-500/10 border-green-500/30" },
];

const auditReports = [
  {
    title: "X3 Kernel Core Audit",
    auditor: "Trail of Bits",
    date: "October 2024",
    status: "Completed",
    findings: "0 Critical, 2 Medium (resolved)",
  },
  {
    title: "EVM Integration Audit",
    auditor: "OpenZeppelin",
    date: "August 2024",
    status: "Completed",
    findings: "0 Critical, 1 High (resolved), 3 Medium (resolved)",
  },
];

export default function SecurityPanel2() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0a100e] to-black text-white p-6 space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-emerald-400 to-green-400 bg-clip-text text-transparent">
          Security
        </h1>
        <p className="text-slate-400 text-sm mt-1">How X3 Chain keeps your assets safe</p>
      </div>

      {/* Status Banner */}
      <div className="bg-emerald-500/10 border border-emerald-500/30 rounded-xl p-5 flex items-center gap-4">
        <CheckCircle className="w-8 h-8 text-emerald-400 flex-shrink-0" />
        <div>
          <div className="font-semibold text-emerald-300 text-lg">All Systems Operational</div>
          <div className="text-sm text-slate-400">
            Last security audit completed October 2024. No outstanding critical issues.
          </div>
        </div>
      </div>

      {/* Security Measures */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Shield className="w-5 h-5 text-emerald-400" />
          Security Measures
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {securityMeasures.map((m) => (
            <div
              key={m.title}
              className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5"
            >
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-lg bg-emerald-500/10">
                  <m.icon className="w-5 h-5 text-emerald-400" />
                </div>
                <h3 className="font-semibold">{m.title}</h3>
              </div>
              <p className="text-sm text-slate-400 leading-relaxed">{m.description}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Bug Bounty */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Bug className="w-5 h-5 text-emerald-400" />
          Bug Bounty Program
        </h2>
        <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-slate-400 border-b border-slate-700/50">
                <th className="px-5 py-3 font-medium">Severity</th>
                <th className="px-5 py-3 font-medium">Reward Range</th>
              </tr>
            </thead>
            <tbody>
              {bountyTiers.map((b) => (
                <tr key={b.severity} className="border-b border-slate-700/30">
                  <td className="px-5 py-3">
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${b.color}`}>
                      {b.severity}
                    </span>
                  </td>
                  <td className="px-5 py-3 font-medium">{b.range}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Vulnerability Reporting */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
          <AlertTriangle className="w-5 h-5 text-yellow-400" />
          Report a Vulnerability
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          If you discover a security vulnerability, please report it responsibly. Do not disclose it publicly
          until we have had a chance to address it. All valid reports are eligible for our bug bounty program.
        </p>
        <div className="flex items-center gap-2 text-emerald-400">
          <Mail className="w-4 h-4" />
          <span className="font-medium">security@x3-chain.io</span>
        </div>
      </div>

      {/* Audit Reports */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <ExternalLink className="w-5 h-5 text-emerald-400" />
          Audit Reports
        </h2>
        <div className="space-y-3">
          {auditReports.map((r, i) => (
            <div
              key={i}
              className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5 flex items-center justify-between"
            >
              <div>
                <h3 className="font-semibold">{r.title}</h3>
                <p className="text-sm text-slate-400 mt-1">
                  By {r.auditor} &middot; {r.date}
                </p>
                <p className="text-xs text-slate-500 mt-1">{r.findings}</p>
              </div>
              <span className="text-xs bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 px-2 py-0.5 rounded-full font-medium">
                {r.status}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
