import React, { useState } from 'react';
import { HelpCircle, MessageSquare, Search, Plus, ChevronDown, CheckCircle } from 'lucide-react';

interface FAQItem {
  id: string;
  question: string;
  answer: string;
  category: string;
  helpful: boolean;
  votes: number;
}

export const FAQSupportPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'faq' | 'contact'>('faq');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [faqs] = useState<FAQItem[]>([
    {
      id: '1',
      question: 'How do I set up a validator node?',
      answer:
        'To set up a validator node, you need to install the X3 software, configure your node with the required hardware specifications, and stake the minimum amount of tokens. Follow the Getting Started guide for detailed instructions.',
      category: 'setup',
      helpful: true,
      votes: 145,
    },
    {
      id: '2',
      question: 'What are the minimum hardware requirements?',
      answer:
        'Minimum requirements include 8GB RAM, 200GB SSD storage, and a stable internet connection with at least 10 Mbps. For optimal performance, we recommend 16GB RAM and 500GB storage.',
      category: 'setup',
      helpful: true,
      votes: 89,
    },
    {
      id: '3',
      question: 'How do I withdraw my staked tokens?',
      answer:
        'You can withdraw staked tokens through the dashboard. Navigate to your Staking section, click Withdraw, and confirm the transaction. Please note that there is a 7-day unstaking period.',
      category: 'staking',
      helpful: true,
      votes: 234,
    },
    {
      id: '4',
      question: 'What rewards do I earn as a validator?',
      answer:
        'Validators earn rewards based on their stake and network participation. Current annual reward rate is approximately 8-12% depending on network conditions. Rewards are distributed automatically.',
      category: 'rewards',
      helpful: true,
      votes: 167,
    },
    {
      id: '5',
      question: 'How do I report a bug or security issue?',
      answer:
        'For security issues, please email security@x3chain.io with details. For bug reports, use our GitHub issue tracker. We take security seriously and offer bug bounties for critical findings.',
      category: 'security',
      helpful: true,
      votes: 92,
    },
    {
      id: '6',
      question: 'Can I run multiple validator nodes?',
      answer:
        'Yes, you can run multiple validator nodes. Each node needs its own stake and hardware resources. However, running validators for the same wallet requires separate configuration files.',
      category: 'setup',
      helpful: true,
      votes: 76,
    },
  ]);

  const categories = ['all', 'setup', 'staking', 'rewards', 'security'];

  const filteredFAQs = faqs.filter((faq) => {
    const matchCategory = selectedCategory === 'all' || faq.category === selectedCategory;
    const matchSearch =
      faq.question.toLowerCase().includes(searchQuery.toLowerCase()) ||
      faq.answer.toLowerCase().includes(searchQuery.toLowerCase());
    return matchCategory && matchSearch;
  });

  const handleToggleExpand = (id: string) => {
    setExpandedId(expandedId === id ? null : id);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Help & Support
            </h1>
            <p className="text-gray-400">Find answers and get support</p>
          </div>
          <HelpCircle className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['faq', 'contact'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'faq' ? 'FAQ' : 'Contact Support'}
            </button>
          ))}
        </div>

        {activeTab === 'faq' ? (
          <div>
            {/* Search */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 mb-6">
              <div className="flex items-center gap-3">
                <Search className="w-5 h-5 text-gray-400" />
                <input
                  type="text"
                  placeholder="Search FAQ..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="flex-1 bg-transparent text-white placeholder-gray-500 focus:outline-none"
                />
              </div>
            </div>

            {/* Categories */}
            <div className="flex gap-2 mb-6 flex-wrap">
              {categories.map((cat) => (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={`px-4 py-2 rounded-lg font-semibold transition ${
                    selectedCategory === cat
                      ? 'bg-cyan-600 text-white'
                      : 'bg-[#1a1a2e] border border-[#2a2a35] text-gray-400 hover:border-cyan-400'
                  }`}
                >
                  {cat.charAt(0).toUpperCase() + cat.slice(1)}
                </button>
              ))}
            </div>

            {/* FAQ List */}
            <div className="space-y-3">
              {filteredFAQs.length === 0 ? (
                <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-8 text-center">
                  <HelpCircle className="w-12 h-12 text-gray-500 mx-auto mb-4 opacity-50" />
                  <p className="text-gray-400">No results found</p>
                </div>
              ) : (
                filteredFAQs.map((faq) => (
                  <div
                    key={faq.id}
                    className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden"
                  >
                    <button
                      onClick={() => handleToggleExpand(faq.id)}
                      className="w-full p-4 flex items-start justify-between hover:bg-[#0a0a0f]/50 transition"
                    >
                      <div className="text-left flex-1">
                        <h3 className="text-white font-semibold mb-1">{faq.question}</h3>
                        <span className="text-xs px-2 py-1 bg-cyan-500/10 text-cyan-400 rounded">
                          {faq.category}
                        </span>
                      </div>
                      <ChevronDown
                        className={`w-5 h-5 text-gray-400 transition transform ${
                          expandedId === faq.id ? 'rotate-180' : ''
                        }`}
                      />
                    </button>

                    {expandedId === faq.id && (
                      <div className="bg-[#0a0a0f] border-t border-[#2a2a35] p-4">
                        <p className="text-gray-300 mb-4">{faq.answer}</p>
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-gray-500">Was this helpful?</span>
                          <div className="flex gap-2">
                            <button className="px-3 py-1 bg-[#1a1a2e] border border-[#2a2a35] text-gray-400 hover:text-green-400 rounded text-xs font-semibold transition">
                              👍 Yes ({faq.votes})
                            </button>
                            <button className="px-3 py-1 bg-[#1a1a2e] border border-[#2a2a35] text-gray-400 hover:text-red-400 rounded text-xs font-semibold transition">
                              👎 No
                            </button>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Contact Form */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <h2 className="text-xl font-bold text-white mb-6">Contact Support</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-gray-400 text-sm font-semibold mb-2">Subject</label>
                  <input
                    type="text"
                    placeholder="How can we help?"
                    className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
                  />
                </div>
                <div>
                  <label className="block text-gray-400 text-sm font-semibold mb-2">Message</label>
                  <textarea
                    placeholder="Describe your issue..."
                    className="w-full h-32 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400 resize-none"
                  />
                </div>
                <button className="w-full px-4 py-3 bg-cyan-600 hover:bg-cyan-700 text-white font-bold rounded-lg transition">
                  Send Message
                </button>
              </div>
            </div>

            {/* Support Channels */}
            <div className="grid grid-cols-2 gap-4">
              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
                <MessageSquare className="w-6 h-6 text-cyan-400 mb-2" />
                <h3 className="text-white font-semibold mb-1">Discord Community</h3>
                <p className="text-gray-400 text-sm mb-3">Join our Discord for real-time support</p>
                <button className="text-cyan-400 hover:text-cyan-300 font-semibold text-sm">Join Server →</button>
              </div>
              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
                <HelpCircle className="w-6 h-6 text-cyan-400 mb-2" />
                <h3 className="text-white font-semibold mb-1">Email Support</h3>
                <p className="text-gray-400 text-sm mb-3">support@x3chain.io</p>
                <button className="text-cyan-400 hover:text-cyan-300 font-semibold text-sm">Send Email →</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default FAQSupportPanel;
