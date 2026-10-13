'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Server,
  Shield,
  Award,
  Coins,
  TrendingUp,
  Clock,
  Users,
  HelpCircle,
  ChevronDown,
  ChevronUp,
  CheckCircle2,
  AlertCircle,
  ExternalLink,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';
import { useAuthorities } from '@/hooks/useSubstrate';

const requirements = [
  { label: 'Minimum Stake', value: '10,000 X3', icon: <Coins className="w-5 h-5" /> },
  { label: 'Hardware', value: '4 CPU, 16GB RAM, 500GB SSD', icon: <Server className="w-5 h-5" /> },
  { label: 'Uptime Requirement', value: '99.5%', icon: <Clock className="w-5 h-5" /> },
  { label: 'Network', value: '100 Mbps dedicated', icon: <TrendingUp className="w-5 h-5" /> },
];

const benefits = [
  {
    title: 'Earn Rewards',
    description: 'Receive block rewards and transaction fees for validating blocks',
    icon: <Coins className="w-6 h-6" />,
    stat: '~8% APY',
  },
  {
    title: 'Governance Rights',
    description: 'Participate in network governance and protocol upgrades',
    icon: <Shield className="w-6 h-6" />,
    stat: '1 Vote per Stake',
  },
  {
    title: 'Network Security',
    description: 'Help secure the X3 STAR network and support decentralization',
    icon: <Award className="w-6 h-6" />,
    stat: '50+ Validators',
  },
  {
    title: 'Community Status',
    description: 'Join the validator community and shape the network\'s future',
    icon: <Users className="w-6 h-6" />,
    stat: 'Exclusive Access',
  },
];

const steps = [
  {
    step: 1,
    title: 'Meet Requirements',
    description: 'Ensure you meet the minimum hardware and stake requirements',
    status: 'required',
  },
  {
    step: 2,
    title: 'Set Up Node',
    description: 'Build and configure the X3 Atlas Sphere node (cargo build --release)',
    status: 'technical',
  },
  {
    step: 3,
    title: 'Generate Keys',
    description: 'Create Aura and GRANDPA session keys for consensus',
    status: 'technical',
  },
  {
    step: 4,
    title: 'Stake X3',
    description: 'Bond your X3 tokens to activate your validator',
    status: 'required',
  },
  {
    step: 5,
    title: 'Start Validating',
    description: 'Begin producing blocks every 6 seconds and earning rewards',
    status: 'final',
  },
];

const faqs = [
  {
    question: 'What is the minimum stake required to become a validator?',
    answer: 'The minimum stake required is 10,000 X3 tokens for testnet. This stake is bonded and can be slashed if the validator misbehaves or goes offline for extended periods.',
  },
  {
    question: 'What are the hardware requirements?',
    answer: 'We recommend at least 4 CPU cores, 16GB RAM, and 500GB SSD storage. You\'ll also need a stable internet connection with at least 100 Mbps bandwidth.',
  },
  {
    question: 'How are rewards calculated?',
    answer: 'X3 Atlas Sphere uses Aura consensus with GRANDPA finality. Rewards are based on block production and your relative stake in the network. Current estimated APY varies based on network activity.',
  },
  {
    question: 'Can I run a validator on cloud infrastructure?',
    answer: 'Yes, you can run validators on cloud providers like AWS, Google Cloud, Hetzner, or DigitalOcean. However, for maximum decentralization, we encourage diverse infrastructure choices.',
  },
  {
    question: 'What consensus mechanism does X3 use?',
    answer: 'X3 Atlas Sphere uses Aura (Authority Round) for block production with 6-second block times, combined with GRANDPA for deterministic finality.',
  },
];

export default function ValidatorsPage() {
  const [openFaq, setOpenFaq] = useState<number | null>(null);
  const { data: authorities, isLoading, error } = useAuthorities();

  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/network" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Network
            </Link>
            <div className="badge badge-success mt-4 mb-4">Network</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Become a Validator
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Secure the X3 Atlas Sphere network, earn rewards, and participate in governance. 
              Join our growing validator community and help decentralize the future.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="#get-started" className="btn-primary">
                Get Started
              </Link>
              <Link href="/developers/docs" className="btn-secondary">
                Read Validator Guide
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Requirements */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Requirements</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {requirements.map((req, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 w-fit mb-4">
                  {req.icon}
                </div>
                <p className="text-sm text-gray-400 mb-1">{req.label}</p>
                <p className="text-xl font-semibold text-white">{req.value}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Benefits */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Validator Benefits</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {benefits.map((benefit, index) => (
              <div key={index} className="glass-card p-6">
                <div className="p-3 rounded-xl bg-gradient-to-br from-orange-500 to-amber-500 bg-opacity-20 w-fit mb-4">
                  <span className="text-white">{benefit.icon}</span>
                </div>
                <h3 className="font-semibold text-white mb-2">{benefit.title}</h3>
                <p className="text-sm text-gray-400 mb-4">{benefit.description}</p>
                <p className="text-lg font-bold gradient-text">{benefit.stat}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Steps */}
      <section id="get-started" className="py-16">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <h2 className="text-2xl font-bold text-white mb-8 text-center">How to Become a Validator</h2>
          
          <div className="space-y-4">
            {steps.map((step, index) => (
              <div key={index} className="glass-card p-6 flex items-center">
                <div className="w-12 h-12 rounded-full bg-orange-500/20 flex items-center justify-center mr-6 flex-shrink-0">
                  <span className="text-xl font-bold text-orange-400">{step.step}</span>
                </div>
                <div className="flex-1">
                  <h3 className="font-semibold text-white mb-1">{step.title}</h3>
                  <p className="text-sm text-gray-400">{step.description}</p>
                </div>
                <div className="ml-4">
                  {step.status === 'final' ? (
                    <CheckCircle2 className="w-6 h-6 text-emerald-500" />
                  ) : (
                    <AlertCircle className="w-6 h-6 text-gray-500" />
                  )}
                </div>
              </div>
            ))}
          </div>
          
          <div className="mt-8 text-center">
            <Link href="/developers/docs" className="btn-primary">
              View Full Validator Guide
            </Link>
          </div>
        </div>
      </section>

      {/* Active Validators */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-bold text-white">Active Validators</h2>
            <Link href="/explorer" className="text-orange-400 hover:text-orange-300 flex items-center">
              View All <ExternalLink className="ml-2 w-4 h-4" />
            </Link>
          </div>
          
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Validator</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Total Stake</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Uptime</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Blocks Produced</th>
                </tr>
              </thead>
              <tbody>
                {error && (
                  <tr>
                    <td colSpan={4} className="py-6 px-4 text-sm text-red-400">
                      Unable to load validators from chain. Please check your node connection.
                    </td>
                  </tr>
                )}

                {!error && isLoading && (
                  Array.from({ length: 4 }).map((_, index) => (
                    <tr key={index} className="border-b border-[#0a0a0a]">
                      <td className="py-4 px-4">
                        <div className="flex items-center">
                          <div className="w-8 h-8 rounded-full bg-[#111111] animate-pulse mr-3" />
                          <div className="h-4 w-40 bg-[#111111] rounded animate-pulse" />
                        </div>
                      </td>
                      <td className="py-4 px-4">
                        <div className="h-4 w-24 bg-[#111111] rounded animate-pulse" />
                      </td>
                      <td className="py-4 px-4">
                        <div className="h-4 w-16 bg-[#111111] rounded animate-pulse" />
                      </td>
                      <td className="py-4 px-4">
                        <div className="h-4 w-24 bg-[#111111] rounded animate-pulse" />
                      </td>
                    </tr>
                  ))
                )}

                {!error && !isLoading && (authorities?.length ?? 0) === 0 && (
                  <tr>
                    <td colSpan={4} className="py-6 px-4 text-sm text-gray-500">
                      No active validators detected on the current network.
                    </td>
                  </tr>
                )}

                {!error && !isLoading && (authorities?.length ?? 0) > 0 &&
                  authorities!.map((validator, index) => {
                    const addr = validator.address;
                    const short = addr.length > 16
                      ? `${addr.slice(0, 8)}…${addr.slice(-6)}`
                      : addr;

                    return (
                      <tr key={addr} className="border-b border-[#0a0a0a] hover:bg-[#0a0a0a]">
                        <td className="py-4 px-4">
                          <div className="flex items-center">
                            <div className="w-8 h-8 rounded-full bg-gradient-to-br from-orange-500 to-amber-500 flex items-center justify-center mr-3">
                              <span className="text-white text-xs font-bold">{index + 1}</span>
                            </div>
                            <div className="flex flex-col">
                              <span className="text-white font-medium">Validator #{index + 1}</span>
                              <span className="text-xs text-gray-500 font-mono">{short}</span>
                            </div>
                          </div>
                        </td>
                        <td className="py-4 px-4 text-gray-400">—</td>
                        <td className="py-4 px-4">
                          <span className="text-emerald-400">{validator.isActive ? 'Active' : 'Inactive'}</span>
                        </td>
                        <td className="py-4 px-4 text-gray-400">—</td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      {/* FAQ */}
      <section className="py-16">
        <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8">
          <h2 className="text-2xl font-bold text-white mb-8 text-center">
            <HelpCircle className="inline w-6 h-6 mr-2" />
            Frequently Asked Questions
          </h2>
          
          <div className="space-y-4">
            {faqs.map((faq, index) => (
              <div key={index} className="glass-card overflow-hidden">
                <button
                  className="w-full p-6 flex items-center justify-between text-left"
                  onClick={() => setOpenFaq(openFaq === index ? null : index)}
                >
                  <span className="font-medium text-white">{faq.question}</span>
                  {openFaq === index ? (
                    <ChevronUp className="w-5 h-5 text-gray-400" />
                  ) : (
                    <ChevronDown className="w-5 h-5 text-gray-400" />
                  )}
                </button>
                {openFaq === index && (
                  <div className="px-6 pb-6">
                    <p className="text-gray-400">{faq.answer}</p>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Ready to Join?
          </h2>
          <p className="text-gray-400 mb-8">
            Start your validator journey today and help secure the X3 Atlas Sphere network.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Validator Setup Guide
            </Link>
            <Link href="/community/forum" className="btn-secondary">
              Join Validator Community
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
