import {
  Users,
  MessageSquare,
  Calendar,
  Gift,
  Globe,
  Star,
  ExternalLink,
  ArrowRight,
} from "lucide-react";

const communityStats = [
  { label: "Discord Members", value: "45K+", color: "text-indigo-400" },
  { label: "GitHub Stars", value: "2.8K", color: "text-slate-300" },
  { label: "Twitter Followers", value: "120K+", color: "text-sky-400" },
  { label: "Active Developers", value: "1,200+", color: "text-pink-400" },
];

const sections = [
  {
    title: "Forum",
    description: "Join discussions, ask questions, and share ideas with the X3 Chain community.",
    icon: MessageSquare,
    gradient: "from-indigo-500 to-purple-500",
  },
  {
    title: "Ecosystem Directory",
    description: "Explore projects, dApps, and tools built on X3 Chain.",
    icon: Globe,
    gradient: "from-cyan-500 to-blue-500",
  },
  {
    title: "Grants Program",
    description: "Apply for funding to build on X3 Chain. Up to $50K per project.",
    icon: Gift,
    gradient: "from-pink-500 to-rose-500",
  },
  {
    title: "Events & Meetups",
    description: "Attend hackathons, workshops, and community calls. Next event: Feb 20.",
    icon: Calendar,
    gradient: "from-orange-500 to-amber-500",
  },
];

const socialLinks = [
  { name: "Discord", members: "45,247 members", color: "bg-indigo-500/20 text-indigo-400 border-indigo-500/30" },
  { name: "Twitter / X", members: "120K followers", color: "bg-sky-500/20 text-sky-400 border-sky-500/30" },
  { name: "GitHub", members: "2,847 stars", color: "bg-slate-500/20 text-slate-300 border-slate-500/30" },
  { name: "Telegram", members: "18,420 members", color: "bg-blue-500/20 text-blue-400 border-blue-500/30" },
];

const featuredProjects = [
  { name: "X3 Swap", stat: "$45M TVL", description: "Decentralized exchange on X3 Chain", tag: "DeFi" },
  { name: "X3 NFT", stat: "10K+ Collections", description: "NFT marketplace and launchpad", tag: "NFT" },
  { name: "Sphere Wallet", stat: "50K+ Users", description: "Mobile-first multi-chain wallet", tag: "Wallet" },
  { name: "X3 Bridge", stat: "$20M Volume", description: "Cross-chain asset bridge", tag: "Infrastructure" },
];

export default function CommunityPanel() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0d0a14] to-black text-white p-6 space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-pink-400 to-rose-400 bg-clip-text text-transparent">
          Community Hub
        </h1>
        <p className="text-slate-400 text-sm mt-1">Connect, build, and grow with X3 Chain</p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {communityStats.map((s) => (
          <div
            key={s.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 text-center"
          >
            <div className={`text-2xl font-bold ${s.color}`}>{s.value}</div>
            <div className="text-xs text-slate-400 mt-1">{s.label}</div>
          </div>
        ))}
      </div>

      {/* Section Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {sections.map((s) => (
          <div
            key={s.title}
            className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5 hover:border-slate-600/50 transition group cursor-pointer"
          >
            <div className="flex items-start gap-4">
              <div className={`p-3 rounded-xl bg-gradient-to-br ${s.gradient} flex-shrink-0`}>
                <s.icon className="w-6 h-6 text-white" />
              </div>
              <div className="flex-1">
                <h3 className="font-semibold text-lg flex items-center gap-2">
                  {s.title}
                  <ArrowRight className="w-4 h-4 text-slate-500 group-hover:text-pink-400 transition" />
                </h3>
                <p className="text-sm text-slate-400 mt-1">{s.description}</p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Social Links */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Users className="w-5 h-5 text-pink-400" />
          Connect With Us
        </h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {socialLinks.map((l) => (
            <div
              key={l.name}
              className={`border rounded-xl p-4 text-center cursor-pointer hover:scale-[1.02] transition ${l.color}`}
            >
              <div className="font-semibold">{l.name}</div>
              <div className="text-xs opacity-80 mt-1">{l.members}</div>
              <ExternalLink className="w-4 h-4 mx-auto mt-2 opacity-50" />
            </div>
          ))}
        </div>
      </div>

      {/* Featured Projects */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Star className="w-5 h-5 text-yellow-400" />
          Featured Projects
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {featuredProjects.map((p) => (
            <div
              key={p.name}
              className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5 hover:border-pink-500/30 transition cursor-pointer"
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-semibold">{p.name}</h3>
                <span className="text-xs bg-pink-500/20 text-pink-400 border border-pink-500/30 px-2 py-0.5 rounded-full">
                  {p.tag}
                </span>
              </div>
              <p className="text-sm text-slate-400">{p.description}</p>
              <div className="mt-3 text-lg font-bold text-pink-400">{p.stat}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
