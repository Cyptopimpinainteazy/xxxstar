'use client';

import React, { useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import dynamic from 'next/dynamic';
import Logo from '../frontend/ui/Logo';

// Dynamic import with SSR disabled to prevent hydration mismatch
const WalletButton = dynamic(
  () => import('@atlas-sphere/shared').then((mod) => mod.WalletButton),
  { 
    ssr: false,
    loading: () => (
      <button className="flex items-center gap-2 px-4 py-2 rounded-xl bg-gradient-to-r from-orange-500 to-red-500 text-white font-medium opacity-80 text-sm" disabled>
        Connect Wallet
      </button>
    )
  }
);

import {
  Menu,
  X,
  ChevronDown,
  Search,
  Globe,
  BookOpen,
  Code,
  Layers,
  Users,
  Boxes,
  Wallet,
  Gamepad2,
  CreditCard,
  Building2,
  Coins,
  Map,
  Smartphone,
  Bot,
  FlaskConical,
  Briefcase,
  Music,
  Server,
  Activity,
  ArrowRightLeft,
  Eye,
  MessageSquare,
  FileText,
  Rocket,
  Cpu,
  Shield,
  Zap,
  Database,
  BarChart3,
  ExternalLink,
  Gift,
  TrendingUp,
} from 'lucide-react';

interface NavItem {
  label: string;
  href?: string;
  icon?: React.ReactNode;
  description?: string;
  children?: NavItem[];
  badge?: string;
}

const navigation: NavItem[] = [
  {
    label: 'Earn',
    href: '/earn',
    badge: 'New',
  },
  {
    label: 'Learn',
    children: [
      {
        label: 'Getting Started',
        href: '/learn/getting-started',
        icon: <Rocket className="w-5 h-5" />,
        description: 'Start your X3 journey here',
      },
      {
        label: 'Tokenomics',
        href: '/learn/tokenomics',
        icon: <Coins className="w-5 h-5" />,
        description: 'How X3Coin powers the ecosystem',
      },
      {
        label: 'Tutorials',
        href: '/learn/tutorials',
        icon: <BookOpen className="w-5 h-5" />,
        description: 'Step-by-step learning guides',
      },
      {
        label: 'Core Concepts',
        href: '/learn/core-concepts',
        icon: <Cpu className="w-5 h-5" />,
        description: 'Understand the fundamentals',
      },
      {
        label: 'Architecture',
        href: '/learn/architecture',
        icon: <Layers className="w-5 h-5" />,
        description: 'Deep dive into X3 design',
      },
    ],
  },
  {
    label: 'Developers',
    children: [
      {
        label: 'Documentation',
        href: '/developers/docs',
        icon: <FileText className="w-5 h-5" />,
        description: 'Official X3 Atlas Sphere documentation',
      },
      {
        label: 'RPC API',
        href: '/developers/api',
        icon: <Code className="w-5 h-5" />,
        description: 'X3 Atlas Sphere RPC API reference',
      },
      {
        label: 'Cookbook',
        href: '/developers/cookbook',
        icon: <BookOpen className="w-5 h-5" />,
        description: 'Snippets and example code',
      },
      {
        label: 'Learning Center',
        href: '/developers/learning',
        icon: <Rocket className="w-5 h-5" />,
        description: 'Start building today',
      },
      {
        label: 'SDKs & Tools',
        href: '/developers/sdks',
        icon: <Boxes className="w-5 h-5" />,
        description: 'Libraries and toolkits',
      },
    ],
  },
  {
    label: 'Solutions',
    children: [
      {
        label: 'Tools',
        href: '/solutions/tools',
        icon: <Boxes className="w-5 h-5" />,
        description: 'Developer tools and utilities',
      },
      {
        label: 'Token Extensions',
        href: '/solutions/token-extensions',
        icon: <Coins className="w-5 h-5" />,
        description: 'Advanced token features',
      },
      {
        label: 'Actions & Blinks',
        href: '/solutions/actions',
        icon: <Zap className="w-5 h-5" />,
        description: 'Blockchain links and actions',
      },
      {
        label: 'Wallets',
        href: '/solutions/wallets',
        icon: <Wallet className="w-5 h-5" />,
        description: 'Wallet solutions',
      },
      {
        label: 'Permissioned Environments',
        href: '/solutions/permissioned',
        icon: <Shield className="w-5 h-5" />,
        description: 'Enterprise blockchain',
      },
      {
        label: 'Games Tooling',
        href: '/solutions/games',
        icon: <Gamepad2 className="w-5 h-5" />,
        description: 'Build blockchain games',
      },
      {
        label: 'Payments Tooling',
        href: '/solutions/payments',
        icon: <CreditCard className="w-5 h-5" />,
        description: 'Payment infrastructure',
      },
      {
        label: 'Commerce Tooling',
        href: '/solutions/commerce',
        icon: <Building2 className="w-5 h-5" />,
        description: 'E-commerce solutions',
      },
      {
        label: 'DeFi',
        href: '/solutions/defi',
        icon: <BarChart3 className="w-5 h-5" />,
        description: 'Decentralized finance',
      },
      {
        label: 'AI & ML',
        href: '/solutions/ai',
        icon: <Bot className="w-5 h-5" />,
        description: 'AI-powered applications',
      },
      {
        label: 'AI Swarm Hub',
        href: '/x3/swarm',
        icon: <Cpu className="w-5 h-5" />,
        description: 'AI agent coordination',
        badge: 'New',
      },
      {
        label: 'Real World Assets',
        href: '/solutions/rwa',
        icon: <Map className="w-5 h-5" />,
        description: 'Tokenize real assets',
      },
      {
        label: 'Mobile',
        href: '/solutions/mobile',
        icon: <Smartphone className="w-5 h-5" />,
        description: 'Mobile blockchain apps',
      },
    ],
  },
  {
    label: 'Network',
    children: [
      {
        label: 'Become a Validator',
        href: '/network/validators',
        icon: <Server className="w-5 h-5" />,
        description: 'Help run the X3 network',
      },
      {
        label: 'RPC Providers',
        href: '/network/rpc-providers',
        icon: <Database className="w-5 h-5" />,
        description: 'Build crypto apps that scale',
      },
      {
        label: 'Network Status',
        href: '/network/status',
        icon: <Activity className="w-5 h-5" />,
        description: 'Network performance and status',
      },
      {
        label: 'On & Off Ramps',
        href: '/network/ramps',
        icon: <ArrowRightLeft className="w-5 h-5" />,
        description: 'Bring your assets to X3',
      },
      {
        label: 'X3scan Explorer',
        href: '/explorer',
        icon: <Eye className="w-5 h-5" />,
        description: 'Explore blockchain in real time',
      },
      {
        label: 'X3FM Indexer',
        href: '/network/indexer',
        icon: <BarChart3 className="w-5 h-5" />,
        description: 'Blockchain explorer and indexer',
      },
      {
        label: 'Prometheus Metrics',
        href: '/prometheus',
        icon: <TrendingUp className="w-5 h-5" />,
        description: 'Live node & kernel metrics',
        badge: 'Live',
      },
    ],
  },
  {
    label: 'Community',
    children: [
      {
        label: 'Forum',
        href: '/community/forum',
        icon: <MessageSquare className="w-5 h-5" />,
        description: 'Join the discussion',
      },
      {
        label: 'Discord',
        href: 'https://discord.gg/x3atlas',
        icon: <Users className="w-5 h-5" />,
        description: 'Chat with the community',
      },
      {
        label: 'Ecosystem',
        href: '/community/ecosystem',
        icon: <Globe className="w-5 h-5" />,
        description: 'Discover X3 projects',
      },
      {
        label: 'Grants',
        href: '/community/grants',
        icon: <Briefcase className="w-5 h-5" />,
        description: 'Funding for builders',
      },
      {
        label: 'Events',
        href: '/community/events',
        icon: <Music className="w-5 h-5" />,
        description: 'Hackathons and meetups',
      },
    ],
  },
  {
    label: 'Explorer',
    href: '/explorer',
  },
];

export default function Navigation() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const pathname = usePathname();

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 10);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled
          ? 'bg-black/95 backdrop-blur-xl border-b border-[#1a1a1a]'
          : 'bg-transparent'
      }`}
    >
      <div className="container-wide">
        <div className="flex items-center justify-between h-20">
          {/* Logo */}
          <Logo size="md" showText={true} />

          {/* Desktop Navigation */}
          <div className="hidden lg:flex items-center space-x-1">
            {navigation.map((item) => (
              <div key={item.label} className="relative group">
                {item.href ? (
                  <Link
                    href={item.href}
                    className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-2 ${
                      pathname === item.href
                        ? 'text-orange-400 bg-orange-500/10'
                        : 'text-gray-400 hover:text-white hover:bg-[#1a1a1a]'
                    }`}
                  >
                    {item.label}
                    {item.badge && (
                      <span className="px-1.5 py-0.5 text-[10px] font-bold bg-gradient-to-r from-orange-500 to-red-500 text-white rounded">
                        {item.badge}
                      </span>
                    )}
                  </Link>
                ) : (
                  <>
                    <button className="flex items-center px-4 py-2 rounded-lg text-sm font-medium text-gray-400 hover:text-white hover:bg-[#1a1a1a] transition-colors">
                      {item.label}
                      <ChevronDown className="ml-1 w-4 h-4" />
                    </button>
                    {item.children && (
                      <div className="menu-dropdown py-2 z-50">
                        <div className="max-h-[70vh] overflow-y-auto custom-scrollbar">
                          {item.children.map((child) => (
                            <Link
                              key={child.label}
                              href={child.href || '#'}
                              className="menu-item flex items-start space-x-3"
                            >
                              <span className="text-orange-500 mt-0.5">
                                {child.icon}
                              </span>
                              <div>
                                <div className="font-medium text-white">
                                  {child.label}
                                </div>
                                {child.description && (
                                  <div className="text-xs text-gray-600">
                                    {child.description}
                                  </div>
                                )}
                              </div>
                            </Link>
                          ))}
                        </div>
                      </div>
                    )}
                  </>
                )}
              </div>
            ))}
          </div>

          {/* Right side actions */}
          <div className="hidden lg:flex items-center space-x-4">
            <button className="p-2 text-gray-500 hover:text-white transition-colors">
              <Search className="w-5 h-5" />
            </button>
            <WalletButton />
            <Link href="/developers/docs" className="btn-secondary text-sm">
              Start Building
            </Link>
          </div>

          {/* Mobile menu button */}
          <button
            className="lg:hidden p-2 text-gray-500 hover:text-white"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            {mobileMenuOpen ? (
              <X className="w-6 h-6" />
            ) : (
              <Menu className="w-6 h-6" />
            )}
          </button>
        </div>
      </div>

      {/* Mobile menu */}
      {mobileMenuOpen && (
        <div className="lg:hidden bg-black/98 backdrop-blur-xl border-t border-[#1a1a1a]">
          <div className="max-h-[80vh] overflow-y-auto custom-scrollbar px-4 py-6 space-y-4">
            {navigation.map((item) => (
              <div key={item.label}>
                {item.href ? (
                  <Link
                    href={item.href}
                    className="block px-4 py-3 text-white font-medium hover:bg-[#1a1a1a] rounded-lg"
                    onClick={() => setMobileMenuOpen(false)}
                  >
                    {item.label}
                  </Link>
                ) : (
                  <div>
                    <div className="px-4 py-2 text-sm font-semibold text-gray-500 uppercase tracking-wider">
                      {item.label}
                    </div>
                    <div className="mt-2 space-y-1">
                      {item.children?.map((child) => (
                        <Link
                          key={child.label}
                          href={child.href || '#'}
                          className="flex items-center space-x-3 px-4 py-3 text-gray-400 hover:text-white hover:bg-[#1a1a1a] rounded-lg"
                          onClick={() => setMobileMenuOpen(false)}
                        >
                          <span className="text-orange-500">{child.icon}</span>
                          <span>{child.label}</span>
                        </Link>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ))}
            <div className="pt-4 border-t border-[#1a1a1a] space-y-3">
              <Link
                href="/developers/docs"
                className="block w-full btn-secondary text-center"
              >
                Start Building
              </Link>
              <Link
                href="https://faucet.testnet.atlas-sphere.io"
                className="block w-full btn-primary text-center"
              >
                Get Testnet Tokens
              </Link>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}
