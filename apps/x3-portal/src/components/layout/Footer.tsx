'use client';

import React from 'react';
import Link from 'next/link';
import Logo from '../frontend/ui/Logo';
import {
  Twitter,
  Github,
  MessageCircle,
  Youtube,
  Globe,
  Heart,
} from 'lucide-react';

const footerLinks = {
  learn: [
    { label: 'Getting Started', href: '/learn/getting-started' },
    { label: 'Tutorials', href: '/learn/tutorials' },
    { label: 'Core Concepts', href: '/learn/core-concepts' },
    { label: 'Architecture', href: '/learn/architecture' },
    { label: 'Whitepaper', href: '/learn/whitepaper' },
  ],
  developers: [
    { label: 'Documentation', href: '/developers/docs' },
    { label: 'RPC API', href: '/developers/api' },
    { label: 'Cookbook', href: '/developers/cookbook' },
    { label: 'SDKs', href: '/developers/sdks' },
    { label: 'GitHub', href: 'https://github.com/atlas-sphere' },
  ],
  solutions: [
    { label: 'DeFi', href: '/solutions/defi' },
    { label: 'Gaming', href: '/solutions/games' },
    { label: 'Payments', href: '/solutions/payments' },
    { label: 'AI & ML', href: '/solutions/ai' },
    { label: 'Enterprise', href: '/solutions/enterprise' },
  ],
  network: [
    { label: 'Explorer', href: '/explorer' },
    { label: 'Validators', href: '/network/validators' },
    { label: 'Network Status', href: '/network/status' },
    { label: 'Testnet Faucet', href: 'https://faucet.testnet.atlas-sphere.io' },
  ],
  community: [
    { label: 'Forum', href: '/community/forum' },
    { label: 'Discord', href: 'https://discord.gg/x3atlas' },
    { label: 'Twitter', href: 'https://twitter.com/x3atlas' },
    { label: 'Blog', href: '/blog' },
    { label: 'Events', href: '/community/events' },
  ],
};

const socialLinks = [
  { icon: <Twitter className="w-5 h-5" />, href: 'https://twitter.com/x3atlas', label: 'Twitter' },
  { icon: <Github className="w-5 h-5" />, href: 'https://github.com/atlas-sphere', label: 'GitHub' },
  { icon: <MessageCircle className="w-5 h-5" />, href: 'https://discord.gg/x3atlas', label: 'Discord' },
  { icon: <Youtube className="w-5 h-5" />, href: 'https://youtube.com/@x3atlas', label: 'YouTube' },
];

export default function Footer() {
  return (
    <footer className="bg-black border-t border-[#1a1a1a]">
      <div className="container-wide py-16">
        {/* Main footer content */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-8 mb-12">
          {/* Brand column */}
          <div className="col-span-2 md:col-span-3 lg:col-span-1">
            <div className="mb-4">
              <Logo size="md" showText={true} />
            </div>
            <p className="text-sm text-gray-500 mb-6">
              Next-generation Layer-1 blockchain with dual VM execution. 
              Bridging EVM and SVM for unprecedented interoperability.
            </p>
            <div className="flex space-x-4">
              {socialLinks.map((social) => (
                <a
                  key={social.label}
                  href={social.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2 text-gray-500 hover:text-orange-400 hover:bg-[#1a1a1a] rounded-lg transition-colors"
                  aria-label={social.label}
                >
                  {social.icon}
                </a>
              ))}
            </div>
          </div>

          {/* Links columns */}
          <div>
            <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
              Learn
            </h3>
            <ul className="space-y-3">
              {footerLinks.learn.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-orange-400 transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
              Developers
            </h3>
            <ul className="space-y-3">
              {footerLinks.developers.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-orange-400 transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
              Solutions
            </h3>
            <ul className="space-y-3">
              {footerLinks.solutions.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-orange-400 transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
              Network
            </h3>
            <ul className="space-y-3">
              {footerLinks.network.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-orange-400 transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
              Community
            </h3>
            <ul className="space-y-3">
              {footerLinks.community.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-orange-400 transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Newsletter signup */}
        <div className="glass-card p-8 mb-12">
          <div className="flex flex-col md:flex-row items-center justify-between gap-6">
            <div>
              <h3 className="text-xl font-bold text-white mb-2">
                Stay updated with X3 Atlas Sphere
              </h3>
              <p className="text-gray-500">
                Get the latest news, updates, and developer resources delivered to your inbox.
              </p>
            </div>
            <div className="flex w-full md:w-auto">
              <input
                type="email"
                placeholder="Enter your email"
                className="flex-1 md:w-64 px-4 py-3 rounded-l-xl bg-[#111111] border border-[#1a1a1a] border-r-0 text-white placeholder-gray-600 focus:outline-none focus:border-orange-500"
              />
              <button className="px-6 py-3 rounded-r-xl bg-gradient-to-r from-orange-600 to-red-600 text-white font-semibold hover:from-orange-500 hover:to-red-500 transition-colors">
                Subscribe
              </button>
            </div>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="flex flex-col md:flex-row items-center justify-between pt-8 border-t border-[#1a1a1a]">
          <div className="flex items-center space-x-1 text-sm text-gray-600 mb-4 md:mb-0">
            <span>© 2025 X3 Atlas Sphere. All rights reserved.</span>
            <span className="mx-2">•</span>
            <span className="flex items-center">
              Built with <Heart className="w-4 h-4 mx-1 text-red-500" /> by the Atlas team
            </span>
          </div>
          <div className="flex items-center space-x-6 text-sm text-gray-600">
            <Link href="/privacy" className="hover:text-orange-400 transition-colors">
              Privacy Policy
            </Link>
            <Link href="/terms" className="hover:text-orange-400 transition-colors">
              Terms of Service
            </Link>
            <Link href="/security" className="hover:text-orange-400 transition-colors">
              Security
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}
