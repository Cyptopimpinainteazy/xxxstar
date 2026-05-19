const blogPosts = [
  {
    slug: "ai-integration",
    title: "AI Integration on X3 Chain",
    excerpt:
      "How decentralized AI inference and fine-tuning workloads run natively on the X3 Chain GPU swarm, enabling permissionless machine learning at scale.",
  },
  {
    slug: "commerce-solutions",
    title: "Commerce Solutions",
    excerpt:
      "End-to-end crypto commerce infrastructure for merchants — from payment processing to loyalty programs, powered by X3 Chain smart contracts.",
  },
  {
    slug: "defi-ecosystem",
    title: "DeFi Ecosystem",
    excerpt:
      "A deep dive into the X3 Chain DeFi stack: automated market makers, lending protocols, yield aggregators, and cross-chain liquidity routing.",
  },
  {
    slug: "gaming-infrastructure",
    title: "Gaming Infrastructure",
    excerpt:
      "Low-latency on-chain gaming primitives, NFT item systems, and provably fair random number generation for next-gen blockchain games.",
  },
  {
    slug: "mobile-integration",
    title: "Mobile Integration",
    excerpt:
      "Building mobile-first dApps with X3 Chain — lightweight SDKs, biometric key management, and seamless wallet connectivity.",
  },
  {
    slug: "payment-systems",
    title: "Payment Systems",
    excerpt:
      "Instant settlement payment rails on X3 Chain, supporting stablecoins, streaming payments, and programmable disbursements.",
  },
  {
    slug: "permissioned-networks",
    title: "Permissioned Networks",
    excerpt:
      "Enterprise-grade permissioned subnets on X3 Chain for regulated industries — compliant, auditable, and interoperable with the public chain.",
  },
  {
    slug: "rwa",
    title: "Real World Assets (RWA)",
    excerpt:
      "Tokenizing real-world assets on X3 Chain: real estate, commodities, bonds, and equities with on-chain compliance and fractional ownership.",
  },
  {
    slug: "token-extensions",
    title: "Token Extensions",
    excerpt:
      "Advanced token standards on X3 Chain — transfer hooks, confidential transfers, interest-bearing tokens, and programmable access controls.",
  },
  {
    slug: "developer-tools",
    title: "Developer Tools",
    excerpt:
      "The X3 Chain developer toolkit: CLI, SDKs, local test validators, block explorers, and IDE integrations for rapid dApp development.",
  },
  {
    slug: "wallet-solutions",
    title: "Wallet Solutions",
    excerpt:
      "Multi-chain wallet infrastructure with social recovery, hardware wallet support, and embedded wallet solutions for frictionless onboarding.",
  },
];

export default function BlogPanel() {
  return (
    <div className="overflow-y-auto h-full bg-[#0a0a0f] text-white p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold">Blog &amp; Resources</h1>
        <p className="text-slate-400 text-sm mt-1">
          Explore features, guides, and deep dives into the X3 Chain ecosystem
        </p>
      </div>

      {/* Card Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
        {blogPosts.map((post) => (
          <div
            key={post.slug}
            className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden hover:border-slate-600/60 transition group cursor-pointer"
          >
            {/* Image Placeholder */}
            <div className="h-36 bg-gradient-to-br from-slate-700/60 to-slate-800/80 flex items-center justify-center">
              <span className="text-3xl font-bold text-slate-600 select-none uppercase tracking-wider">
                {post.slug
                  .split("-")
                  .map((w) => w[0])
                  .join("")}
              </span>
            </div>

            {/* Content */}
            <div className="p-4 space-y-2">
              <h3 className="font-semibold text-slate-100 group-hover:text-orange-400 transition">
                {post.title}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed line-clamp-3">{post.excerpt}</p>
              <span className="inline-block text-xs text-orange-400 font-medium mt-1 cursor-pointer">
                Read more →
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
