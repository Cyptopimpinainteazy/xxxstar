'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  Gift,
  DollarSign,
  CheckCircle2,
  Clock,
  FileText,
  Users,
  Zap,
  Shield,
  Globe,
  ArrowRight,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const grantTracks = [
  {
    name: 'Infrastructure',
    description: 'Build core infrastructure, tooling, and developer experience',
    funding: 'Up to $100K',
    icon: <Shield className="w-6 h-6" />,
    examples: ['RPC providers', 'Indexers', 'Developer tools', 'SDKs'],
    color: 'from-orange-500 to-amber-500',
  },
  {
    name: 'DeFi',
    description: 'Create DeFi protocols leveraging cross-VM capabilities',
    funding: 'Up to $75K',
    icon: <DollarSign className="w-6 h-6" />,
    examples: ['DEXes', 'Lending', 'Yield', 'Derivatives'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    name: 'Consumer Apps',
    description: 'Build user-facing applications that showcase X3 STAR',
    funding: 'Up to $50K',
    icon: <Users className="w-6 h-6" />,
    examples: ['Wallets', 'Social apps', 'Games', 'NFT platforms'],
    color: 'from-blue-500 to-cyan-500',
  },
  {
    name: 'Research',
    description: 'Conduct research on cross-VM technology and blockchain',
    funding: 'Up to $30K',
    icon: <FileText className="w-6 h-6" />,
    examples: ['Security audits', 'Economic research', 'Technical papers'],
    color: 'from-purple-500 to-indigo-500',
  },
];

const recentGrants = [
  {
    project: 'X3 Indexer',
    category: 'Infrastructure',
    amount: '$85,000',
    status: 'completed',
    description: 'High-performance blockchain indexer for dApp developers',
  },
  {
    project: 'DualPool AMM',
    category: 'DeFi',
    amount: '$60,000',
    status: 'in-progress',
    description: 'Cross-VM automated market maker',
  },
  {
    project: 'Atlas Mobile Wallet',
    category: 'Consumer Apps',
    amount: '$45,000',
    status: 'in-progress',
    description: 'Mobile-first wallet for X3 STAR',
  },
  {
    project: 'Cross-VM Security Analysis',
    category: 'Research',
    amount: '$25,000',
    status: 'completed',
    description: 'Security analysis of cross-VM transaction patterns',
  },
];

const processSteps = [
  {
    step: 1,
    title: 'Submit Application',
    description: 'Fill out our grant application form with your project details',
  },
  {
    step: 2,
    title: 'Initial Review',
    description: 'Our team reviews your application within 2 weeks',
  },
  {
    step: 3,
    title: 'Interview',
    description: 'Selected projects are invited for a deeper discussion',
  },
  {
    step: 4,
    title: 'Committee Decision',
    description: 'Grant committee makes final funding decisions',
  },
  {
    step: 5,
    title: 'Milestone-Based Funding',
    description: 'Receive funding as you hit agreed milestones',
  },
];

const faqs = [
  {
    question: 'Who can apply for a grant?',
    answer: 'Anyone! We welcome applications from individuals, teams, and organizations worldwide. Whether you\'re a solo developer or a large team, if you have a great idea that benefits the X3 STAR ecosystem, we want to hear from you.',
  },
  {
    question: 'What kind of projects get funded?',
    answer: 'We fund projects that improve the X3 STAR ecosystem, including infrastructure tools, DeFi protocols, consumer applications, educational content, and research. Priority is given to projects that leverage the unique cross-VM capabilities of X3 STAR.',
  },
  {
    question: 'How long does the application process take?',
    answer: 'Initial review takes about 2 weeks. If selected for interview, the full process typically takes 4-6 weeks from application to funding decision.',
  },
  {
    question: 'How is funding distributed?',
    answer: 'Funding is distributed in milestones. You\'ll receive an initial payment upon approval, with subsequent payments tied to achieving agreed-upon milestones.',
  },
  {
    question: 'Do I need to open source my project?',
    answer: 'Not necessarily. While we encourage open source development, we also fund projects with proprietary components. The specific requirements depend on the grant track and project type.',
  },
];

const stats = [
  { label: 'Total Funded', value: '$2.5M+' },
  { label: 'Projects Funded', value: '45' },
  { label: 'Success Rate', value: '32%' },
  { label: 'Avg Grant Size', value: '$55K' },
];

export default function GrantsPage() {
  const [openFaq, setOpenFaq] = useState<number | null>(null);

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
            <Link href="/community" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Community
            </Link>
            <div className="badge badge-success mt-4 mb-4">Grants</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR Grants Program
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Get funding to build on X3 STAR. We support developers, researchers, and 
              teams building the future of cross-VM blockchain technology.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#apply" className="btn-primary">
                Apply for Grant
              </a>
              <a href="#tracks" className="btn-secondary">
                View Grant Tracks
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {stats.map((stat, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <p className="text-3xl font-bold gradient-text">{stat.value}</p>
                <p className="text-sm text-gray-400 mt-2">{stat.label}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Grant Tracks */}
      <section id="tracks" className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Grant Tracks</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {grantTracks.map((track, index) => (
              <div key={index} className="glass-card p-6">
                <div className={`p-3 rounded-xl bg-gradient-to-br ${track.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{track.icon}</span>
                </div>
                <div className="flex items-center justify-between mb-2">
                  <h3 className="text-xl font-semibold text-white">{track.name}</h3>
                  <span className="text-emerald-400 font-medium">{track.funding}</span>
                </div>
                <p className="text-gray-400 mb-4">{track.description}</p>
                <div className="flex flex-wrap gap-2">
                  {track.examples.map((example, i) => (
                    <span key={i} className="badge badge-default">{example}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Process */}
      <section className="py-16 bg-[#050505]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <h2 className="text-2xl font-bold text-white mb-8 text-center">Application Process</h2>
          
          <div className="space-y-4">
            {processSteps.map((step, index) => (
              <div key={index} className="glass-card p-6 flex items-center">
                <div className="w-12 h-12 rounded-full bg-orange-500/20 flex items-center justify-center mr-6 flex-shrink-0">
                  <span className="text-xl font-bold text-orange-400">{step.step}</span>
                </div>
                <div>
                  <h3 className="font-semibold text-white mb-1">{step.title}</h3>
                  <p className="text-sm text-gray-400">{step.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Recent Grants */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Recently Funded Projects</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {recentGrants.map((grant, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <span className="badge badge-default">{grant.category}</span>
                  <span className={`badge ${grant.status === 'completed' ? 'badge-success' : 'badge-warning'}`}>
                    {grant.status}
                  </span>
                </div>
                <h3 className="font-semibold text-white mb-2">{grant.project}</h3>
                <p className="text-sm text-gray-400 mb-4">{grant.description}</p>
                <p className="text-lg font-bold gradient-text">{grant.amount}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* FAQ */}
      <section className="py-16 bg-[#050505]">
        <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8">
          <h2 className="text-2xl font-bold text-white mb-8 text-center">
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

      {/* Apply CTA */}
      <section id="apply" className="py-16">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="glass-card p-8 md:p-12 text-center">
            <Gift className="w-12 h-12 text-orange-400 mx-auto mb-6" />
            <h2 className="text-3xl font-bold text-white mb-4">
              Ready to Apply?
            </h2>
            <p className="text-gray-400 mb-8 max-w-2xl mx-auto">
              Submit your grant application today and join the builders shaping the 
              future of X3 STAR. Our team is here to support you every step of the way.
            </p>
            <div className="flex flex-wrap justify-center gap-4">
              <a href="#" className="btn-primary">
                Start Application
              </a>
              <a href="#" className="btn-secondary">
                Schedule Office Hours
              </a>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
