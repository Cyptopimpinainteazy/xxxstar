'use client';

import React from 'react';
import Link from 'next/link';
import { Shield, Lock, Bug, AlertTriangle, CheckCircle, Mail, ExternalLink } from 'lucide-react';

export default function SecurityPage() {
  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a]">
        <div className="absolute inset-0 bg-gradient-to-br from-emerald-950/20 via-black to-green-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="relative z-10 max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="inline-flex items-center px-3 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 mb-4">
            <Shield className="w-4 h-4 mr-2 text-emerald-400" />
            <span className="text-sm text-emerald-300">Security First</span>
          </div>
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">Security</h1>
          <p className="text-xl text-gray-500">
            Learn about our security practices and how to report vulnerabilities
          </p>
        </div>
      </section>

      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        {/* Security Status */}
        <div className="glass-card p-6 mb-12 border-emerald-500/20">
          <div className="flex items-center space-x-4">
            <div className="w-12 h-12 rounded-xl bg-emerald-500/20 flex items-center justify-center">
              <CheckCircle className="w-6 h-6 text-emerald-400" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-white">All Systems Operational</h2>
              <p className="text-sm text-gray-500">Last security audit: October 2024</p>
            </div>
          </div>
        </div>

        {/* Security Measures */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-6">Security Measures</h2>
          <div className="grid md:grid-cols-2 gap-4">
            <div className="glass-card p-4">
              <Lock className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Authorization System</h3>
              <p className="text-sm text-gray-500">
                Comit submissions require explicit account authorization through governance
              </p>
            </div>
            <div className="glass-card p-4">
              <Shield className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Atomic Execution</h3>
              <p className="text-sm text-gray-500">
                Cross-VM operations are atomic - preventing partial execution states
              </p>
            </div>
            <div className="glass-card p-4">
              <CheckCircle className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Prepare Root Verification</h3>
              <p className="text-sm text-gray-500">
                All Comits are verified against their prepare_root hash before finalization
              </p>
            </div>
            <div className="glass-card p-4">
              <Bug className="w-6 h-6 text-orange-400 mb-2" />
              <h3 className="font-semibold text-white mb-1">Third-Party Audits</h3>
              <p className="text-sm text-gray-500">
                Regular security audits by leading blockchain security firms
              </p>
            </div>
          </div>
        </section>

        {/* Bug Bounty */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-4">Bug Bounty Program</h2>
          <div className="glass-card p-6 bg-gradient-to-r from-orange-500/10 to-red-500/10 border-orange-500/20">
            <p className="text-gray-400 mb-4">
              We offer rewards for responsibly disclosed security vulnerabilities. 
              Rewards range from $500 to $50,000 depending on severity.
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-[#1a1a1a]">
                    <th className="text-left py-2 px-4 text-gray-500">Severity</th>
                    <th className="text-left py-2 px-4 text-gray-500">Reward</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b border-[#111111]">
                    <td className="py-2 px-4 text-red-400">Critical</td>
                    <td className="py-2 px-4 text-white">$25,000 - $50,000</td>
                  </tr>
                  <tr className="border-b border-[#111111]">
                    <td className="py-2 px-4 text-orange-400">High</td>
                    <td className="py-2 px-4 text-white">$10,000 - $25,000</td>
                  </tr>
                  <tr className="border-b border-[#111111]">
                    <td className="py-2 px-4 text-amber-400">Medium</td>
                    <td className="py-2 px-4 text-white">$2,500 - $10,000</td>
                  </tr>
                  <tr>
                    <td className="py-2 px-4 text-emerald-400">Low</td>
                    <td className="py-2 px-4 text-white">$500 - $2,500</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>

        {/* Reporting */}
        <section className="mb-12">
          <h2 className="text-2xl font-bold text-white mb-4">Report a Vulnerability</h2>
          <div className="glass-card p-6">
            <AlertTriangle className="w-8 h-8 text-amber-400 mb-4" />
            <p className="text-gray-400 mb-4">
              If you discover a security vulnerability, please report it responsibly:
            </p>
            <ul className="space-y-3 mb-6">
              <li className="flex items-center text-gray-400">
                <Mail className="w-5 h-5 text-orange-400 mr-2" />
                Email: <a href="mailto:security@atlas-sphere.io" className="text-orange-400 hover:text-orange-300 ml-1">security@atlas-sphere.io</a>
              </li>
              <li className="flex items-start text-gray-400">
                <CheckCircle className="w-5 h-5 text-emerald-400 mr-2 mt-0.5" />
                <span>Include detailed steps to reproduce the vulnerability</span>
              </li>
              <li className="flex items-start text-gray-400">
                <CheckCircle className="w-5 h-5 text-emerald-400 mr-2 mt-0.5" />
                <span>Allow us 90 days to address the issue before public disclosure</span>
              </li>
            </ul>
            <p className="text-sm text-gray-500">
              PGP Key: Available at <a href="/security/pgp-key.txt" className="text-orange-400">security/pgp-key.txt</a>
            </p>
          </div>
        </section>

        {/* Audits */}
        <section>
          <h2 className="text-2xl font-bold text-white mb-4">Security Audits</h2>
          <div className="space-y-4">
            <div className="glass-card p-4 flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-white">Atlas Kernel Pallet Audit</h3>
                <p className="text-sm text-gray-500">By Trail of Bits - October 2024</p>
              </div>
              <Link href="/audits/atlas-kernel-2024.pdf" className="btn-secondary text-sm flex items-center">
                View Report <ExternalLink className="w-4 h-4 ml-1" />
              </Link>
            </div>
            <div className="glass-card p-4 flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-white">EVM Integration Audit</h3>
                <p className="text-sm text-gray-500">By OpenZeppelin - September 2024</p>
              </div>
              <Link href="/audits/evm-integration-2024.pdf" className="btn-secondary text-sm flex items-center">
                View Report <ExternalLink className="w-4 h-4 ml-1" />
              </Link>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
