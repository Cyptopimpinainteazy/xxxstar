import { Metadata } from 'next'
import Link from 'next/link'

export const metadata: Metadata = {
  title: 'X3 Atlas Ecosystem - Revolutionizing Blockchain Infrastructure',
  description: 'Explore how X3 Atlas is redefining blockchain technology through cutting-edge infrastructure solutions and unparalleled ecosystem integration',
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: 'https://x3atlas.xyz/blog',
    title: 'X3 Atlas Ecosystem - Blockchain Innovation Ledger',
    description: 'Next-gen blockchain solutions powering enterprise adoption and web3 evolution',
    images: [
      {
        url: '/blog/og-image.jpg', // USER PROVIDED IMAGE NEEDED
        width: 1200,
        height: 630,
        alt: 'X3 Atlas Ecosystem',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'X3 Atlas Ecosystem - Blockchain Innovation Ledger',
    description: 'Next-gen blockchain solutions powering enterprise adoption and web3 evolution',
    images: ['/blog/og-image.jpg'], // USER PROVIDED IMAGE NEEDED
  },
}

export default function Blog() {
  const features = [
    { 
      slug: 'ai-integration',
      title: 'AI-Powered Blockchain Orchestration',
      excerpt: 'Discover how X3 Atlas leverages artificial intelligence to create self-optimizing smart contracts and predictive network management'
    },
    {
      slug: 'commerce-solutions',
      title: 'Enterprise-Grade Commerce Infrastructure',
      excerpt: 'Explore our merchant-ready payment rails with instant settlement and zero fraud risk'
    },
    {
      slug: 'defi-ecosystem',
      title: 'Institutional DeFi Architecture',
      excerpt: 'Multi-chain liquidity aggregation meets compliance-ready financial primitives'
    },
    {
      slug: 'gaming-infrastructure',
      title: 'Web3 Gaming Superstructure',
      excerpt: 'Battle-tested infrastructure for play-to-earn economies and NFT gaming ecosystems'
    },
    {
      slug: 'mobile-integration',
      title: 'Mobile-First Blockchain OS',
      excerpt: 'Secure mobile gateway to multi-chain assets with carrier-grade performance'
    },
    {
      slug: 'payment-systems',
      title: 'Global Payments Network',
      excerpt: 'Frictionless cross-border transactions powered by hybrid blockchain architecture'
    },
    {
      slug: 'permissioned-networks',
      title: 'Regulation-Ready Private Chains',
      excerpt: 'Enterprise blockchain solutions with built-in compliance controls'
    },
    {
      slug: 'rwa',
      title: 'Real World Asset Tokenization',
      excerpt: 'Institutional-grade infrastructure for digital securities and asset-backed tokens'
    },
    {
      slug: 'token-extensions',
      title: 'Smart Token Standards',
      excerpt: 'Programmable digital assets with embedded compliance and cross-chain superpowers'
    },
    {
      slug: 'developer-tools',
      title: 'Full-Stack Web3 Toolkit',
      excerpt: 'Everything developers need to build on the next generation of blockchain infrastructure'
    },
    {
      slug: 'wallet-solutions',
      title: 'Enterprise Wallet Architecture',
      excerpt: 'Institutional-grade custody solutions meets consumer-friendly UX'
    }
  ]

  return (
    <div className="max-w-7xl mx-auto px-4 py-12">
      <header className="mb-16 text-center">
        <h1 className="text-5xl font-bold mb-4">
          X3 Atlas Blockchain Innovation Hub
        </h1>
        <p className="text-xl text-gray-600">
          Deep technical analysis of the infrastructure shaping Web3's future
        </p>
      </header>

      <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
        {features.map((feature) => (
          <Link 
            key={feature.slug}
            href={`/blog/${feature.slug}`}
            className="border rounded-lg p-6 hover:shadow-lg transition-shadow"
          >
            <div className="aspect-video bg-gray-100 mb-4 rounded-lg">{/* IMAGE PLACEHOLDER */}</div>
            <h2 className="text-2xl font-semibold mb-2">{feature.title}</h2>
            <p className="text-gray-600">{feature.excerpt}</p>
          </Link>
        ))}
      </div>
    </div>
  )
}
