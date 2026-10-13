'use client';

import React from 'react';
import Link from 'next/link';
import {
  Users,
  MessageSquare,
  Calendar,
  Gift,
  Globe,
  Twitter,
  Github,
  ArrowRight,
  ExternalLink,
  Heart,
  Star,
  Zap,
} from 'lucide-react';
import { HexagonCluster } from '../../components/frontend/ui/Logo';

const communityStats = [
  { label: 'Discord Members', value: '45K+', icon: <MessageSquare className="w-5 h-5" /> },
  { label: 'GitHub Stars', value: '2.8K', icon: <Star className="w-5 h-5" /> },
  { label: 'Twitter Followers', value: '120K', icon: <Twitter className="w-5 h-5" /> },
  { label: 'Active Developers', value: '1,200+', icon: <Users className="w-5 h-5" /> },
];

const sections = [
  {
    title: 'Community Forum',
    description: 'Discuss ideas, ask questions, and connect with other X3 STAR builders',
    icon: <MessageSquare className="w-8 h-8" />,
    href: '/community/forum',
    color: 'from-orange-500 to-amber-500',
  },
  {
    title: 'Ecosystem Directory',
    description: 'Discover projects, dApps, and services built on X3 STAR',
    icon: <Globe className="w-8 h-8" />,
    href: '/community/ecosystem',
    color: 'from-blue-500 to-cyan-500',
  },
  {
    title: 'Grants Program',
    description: 'Get funding to build on X3 STAR and grow the ecosystem',
    icon: <Gift className="w-8 h-8" />,
    href: '/community/grants',
    color: 'from-emerald-500 to-green-500',
  },
  {
    title: 'Events',
    description: 'Hackathons, meetups, conferences, and community calls',
    icon: <Calendar className="w-8 h-8" />,
    href: '/community/events',
    color: 'from-purple-500 to-indigo-500',
  },
];

const socialLinks = [
  {
    name: 'Discord',
    description: 'Join our Discord server for real-time discussions and support',
    url: '#',
    icon: <MessageSquare className="w-6 h-6" />,
    members: '45,000+ members',
  },
  {
    name: 'Twitter',
    description: 'Follow us for the latest news and announcements',
    url: '#',
    icon: <Twitter className="w-6 h-6" />,
    members: '120,000+ followers',
  },
  {
    name: 'GitHub',
    description: 'Contribute to X3 STAR open source projects',
    url: '#',
    icon: <Github className="w-6 h-6" />,
    members: '500+ contributors',
  },
  {
    name: 'Telegram',
    description: 'Join our Telegram group for quick updates',
    url: '#',
    icon: <MessageSquare className="w-6 h-6" />,
    members: '30,000+ members',
  },
];

const featuredProjects = [
  {
    name: 'X3 Swap',
    category: 'DeFi',
    description: 'Cross-VM decentralized exchange',
    tvl: '$45M TVL',
  },
  {
    name: 'Atlas NFT',
    category: 'NFT',
    description: 'NFT marketplace for digital collectibles',
    tvl: '10K+ Collections',
  },
  {
    name: 'Sphere Wallet',
    category: 'Wallet',
    description: 'Multi-VM wallet for X3 STAR',
    tvl: '50K+ Users',
  },
  {
    name: 'X3 Bridge',
    category: 'Infrastructure',
    description: 'Cross-chain bridge to other networks',
    tvl: '$20M Volume',
  },
];

const upcomingEvents = [
  {
    title: 'X3 STAR Hackathon',
    date: 'Feb 15-17, 2024',
    type: 'Hackathon',
    location: 'Virtual',
    prize: '$50,000',
  },
  {
    title: 'Community Call #24',
    date: 'Jan 25, 2024',
    type: 'Call',
    location: 'Discord',
    prize: null,
  },
  {
    title: 'ETH Denver Workshop',
    date: 'Feb 28, 2024',
    type: 'Workshop',
    location: 'Denver, CO',
    prize: null,
  },
];

export default function CommunityPage() {
  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero with unique Community section header */}
      <section className="py-20 relative overflow-hidden page-header-community">
        <div className="absolute inset-0 bg-gradient-to-br from-pink-950/20 via-black to-rose-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-20">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="inline-flex items-center px-3 py-1 rounded-full bg-pink-500/10 border border-pink-500/30 mb-4">
              <Users className="w-4 h-4 mr-2 text-pink-400" />
              <span className="text-sm text-pink-300">Community</span>
            </div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Join the <span className="text-pink-400">X3 STAR Community</span>
            </h1>
            <p className="text-xl text-gray-500 mb-8">
              Connect with developers, validators, and enthusiasts building the future 
              of cross-VM blockchain technology.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#" className="btn-primary">
                <MessageSquare className="w-4 h-4 mr-2" />
                Join Discord
              </a>
              <Link href="/community/forum" className="btn-secondary">
                Visit Forum
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a] bg-black">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {communityStats.map((stat, index) => (
              <div key={index} className="glass-card p-6 flex items-center">
                <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400 mr-4">
                  {stat.icon}
                </div>
                <div>
                  <p className="text-sm text-gray-500">{stat.label}</p>
                  <p className="text-2xl font-bold text-white">{stat.value}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Sections */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {sections.map((section, index) => (
              <Link
                key={index}
                href={section.href}
                className="glass-card-hover p-6 card-lift group"
              >
                <div className={`p-3 rounded-xl bg-gradient-to-br ${section.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{section.icon}</span>
                </div>
                <h3 className="text-lg font-semibold text-white mb-2 group-hover:text-orange-400 transition-colors">
                  {section.title}
                </h3>
                <p className="text-sm text-gray-500">{section.description}</p>
              </Link>
            ))}
          </div>
        </div>
      </section>

      {/* Social Links */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Connect With Us</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {socialLinks.map((social, index) => (
              <a
                key={index}
                href={social.url}
                className="glass-card p-6 hover:border-orange-500/30 transition-colors group"
              >
                <div className="flex items-center justify-between mb-4">
                  <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400">
                    {social.icon}
                  </div>
                  <ExternalLink className="w-4 h-4 text-gray-600 group-hover:text-orange-400 transition-colors" />
                </div>
                <h3 className="font-semibold text-white mb-2 group-hover:text-orange-400 transition-colors">
                  {social.name}
                </h3>
                <p className="text-sm text-gray-500 mb-2">{social.description}</p>
                <p className="text-xs text-gray-600">{social.members}</p>
              </a>
            ))}
          </div>
        </div>
      </section>

      {/* Featured Projects */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-bold text-white">Featured Projects</h2>
            <Link href="/community/ecosystem" className="text-orange-400 hover:text-orange-300 flex items-center">
              View All <ArrowRight className="ml-2 w-4 h-4" />
            </Link>
          </div>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {featuredProjects.map((project, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <span className="badge badge-default">{project.category}</span>
                </div>
                <h3 className="font-semibold text-white mb-2">{project.name}</h3>
                <p className="text-sm text-gray-500 mb-4">{project.description}</p>
                <p className="text-sm font-medium gradient-text">{project.tvl}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Upcoming Events */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-bold text-white">Upcoming Events</h2>
            <Link href="/community/events" className="text-orange-400 hover:text-orange-300 flex items-center">
              View All <ArrowRight className="ml-2 w-4 h-4" />
            </Link>
          </div>
          
          <div className="grid md:grid-cols-3 gap-6">
            {upcomingEvents.map((event, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <span className={`badge ${event.type === 'Hackathon' ? 'badge-success' : event.type === 'Workshop' ? 'badge-info' : 'badge-default'}`}>
                    {event.type}
                  </span>
                  {event.prize && (
                    <span className="text-sm font-medium gradient-text">{event.prize}</span>
                  )}
                </div>
                <h3 className="font-semibold text-white mb-2">{event.title}</h3>
                <div className="space-y-1 text-sm text-gray-500">
                  <p className="flex items-center">
                    <Calendar className="w-4 h-4 mr-2" />
                    {event.date}
                  </p>
                  <p className="flex items-center">
                    <Globe className="w-4 h-4 mr-2" />
                    {event.location}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Contribute */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <div className="glass-card p-8 md:p-12 text-center">
            <Heart className="w-12 h-12 text-red-500 mx-auto mb-6" />
            <h2 className="text-3xl font-bold text-white mb-4">
              Contribute to X3 STAR
            </h2>
            <p className="text-gray-500 mb-8 max-w-2xl mx-auto">
              X3 STAR is open source and community-driven. Whether you're a developer, 
              designer, writer, or enthusiast, there are many ways to contribute.
            </p>
            <div className="flex flex-wrap justify-center gap-4">
              <a href="#" className="btn-primary">
                <Github className="w-4 h-4 mr-2" />
                Contribute on GitHub
              </a>
              <Link href="/community/grants" className="btn-secondary">
                Apply for Grant
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Newsletter */}
      <section className="py-16 border-t border-[#1a1a1a] bg-black">
        <div className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Zap className="w-10 h-10 text-orange-400 mx-auto mb-4" />
          <h2 className="text-2xl font-bold text-white mb-4">
            Stay Updated
          </h2>
          <p className="text-gray-500 mb-6">
            Subscribe to our newsletter for the latest news, updates, and community highlights.
          </p>
          <form className="flex gap-3 max-w-md mx-auto">
            <input
              type="email"
              placeholder="Enter your email"
              className="flex-1 px-4 py-3 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-600 focus:border-orange-500/50 focus:outline-none"
            />
            <button type="submit" className="btn-primary whitespace-nowrap">
              Subscribe
            </button>
          </form>
        </div>
      </section>
    </div>
  );
}
