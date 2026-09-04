import React, { useState } from 'react';
import { Database, Users, Mail, BarChart3, TrendingUp, Download } from 'lucide-react';

interface Contact {
  id: string;
  name: string;
  email: string;
  phone: string;
  company: string;
  status: 'active' | 'inactive' | 'lead';
  lastContact: number;
  tags: string[];
}

interface Deal {
  id: string;
  name: string;
  value: number;
  stage: 'prospecting' | 'qualification' | 'proposal' | 'negotiation' | 'closed-won' | 'closed-lost';
  probability: number;
  owner: string;
  createdDate: number;
  expectedClose: number;
  contacts: string[];
}

interface EmailCampaign {
  id: string;
  name: string;
  subject: string;
  recipients: number;
  sent: number;
  opened: number;
  clicked: number;
  bounced: number;
  status: 'draft' | 'sending' | 'sent' | 'paused';
}

interface CrmMetrics {
  totalContacts: number;
  activeDeals: number;
  totalPipeline: number;
  conversionRate: number;
  avgDealSize: number;
  avgSalesAge: number;
}

export const CRMDatabasePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'contacts' | 'deals' | 'campaigns' | 'metrics'>('contacts');
  const [contacts] = useState<Contact[]>([
    {
      id: 'c1',
      name: 'Alice Johnson',
      email: 'alice@techcorp.com',
      phone: '+1-555-0101',
      company: 'TechCorp Inc',
      status: 'active',
      lastContact: Date.now() - 86400000 * 2,
      tags: ['enterprise', 'blockchain', 'hot-lead'],
    },
    {
      id: 'c2',
      name: 'Bob Smith',
      email: 'bob@finservices.io',
      phone: '+1-555-0102',
      company: 'FinServices Ltd',
      status: 'active',
      lastContact: Date.now() - 86400000 * 5,
      tags: ['finance', 'crypto', 'expansion'],
    },
    {
      id: 'c3',
      name: 'Carol Davis',
      email: 'carol@devstudio.co',
      phone: '+1-555-0103',
      company: 'DevStudio Co',
      status: 'lead',
      lastContact: Date.now() - 86400000 * 12,
      tags: ['developer', 'api-integration', 'starter'],
    },
  ]);

  const [deals] = useState<Deal[]>([
    {
      id: 'd1',
      name: 'Enterprise Integration - TechCorp',
      value: 250000,
      stage: 'proposal',
      probability: 75,
      owner: 'Sarah Chen',
      createdDate: Date.now() - 86400000 * 30,
      expectedClose: Date.now() + 86400000 * 15,
      contacts: ['c1'],
    },
    {
      id: 'd2',
      name: 'Trading Platform Deployment',
      value: 180000,
      stage: 'negotiation',
      probability: 60,
      owner: 'Mike Torres',
      createdDate: Date.now() - 86400000 * 45,
      expectedClose: Date.now() + 86400000 * 25,
      contacts: ['c2'],
    },
    {
      id: 'd3',
      name: 'API Development & Support',
      value: 45000,
      stage: 'qualification',
      probability: 40,
      owner: 'Jessica Park',
      createdDate: Date.now() - 86400000 * 10,
      expectedClose: Date.now() + 86400000 * 45,
      contacts: ['c3'],
    },
  ]);

  const [emailCampaigns] = useState<EmailCampaign[]>([
    {
      id: 'e1',
      name: 'Q1 2026 Product Launch',
      subject: 'Introducing X3 Network - Next-Gen Blockchain',
      recipients: 4500,
      sent: 4450,
      opened: 1780,
      clicked: 445,
      bounced: 50,
      status: 'sent',
    },
    {
      id: 'e2',
      name: 'Enterprise Features Webinar',
      subject: 'Join Our Live Webinar: Enterprise Scaling',
      recipients: 2300,
      sent: 2280,
      opened: 850,
      clicked: 220,
      bounced: 20,
      status: 'sent',
    },
    {
      id: 'e3',
      name: 'Spring Promotion Campaign',
      subject: '40% Off Enterprise Plans - Limited Time',
      recipients: 6000,
      sent: 2100,
      opened: 0,
      clicked: 0,
      bounced: 0,
      status: 'sending',
    },
  ]);

  const [metrics] = useState<CrmMetrics>({
    totalContacts: 2840,
    activeDeals: 18,
    totalPipeline: 4750000,
    conversionRate: 23.5,
    avgDealSize: 263889,
    avgSalesAge: 34,
  });

  const stageColors: Record<Deal['stage'], string> = {
    'prospecting': 'bg-blue-500/20 text-blue-400',
    'qualification': 'bg-cyan-500/20 text-cyan-400',
    'proposal': 'bg-purple-500/20 text-purple-400',
    'negotiation': 'bg-orange-500/20 text-orange-400',
    'closed-won': 'bg-green-500/20 text-green-400',
    'closed-lost': 'bg-red-500/20 text-red-400',
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-orange-400 to-red-500 mb-2">
              CRM Database
            </h1>
            <p className="text-gray-400">Contacts • Deals Pipeline • Email Campaigns • Contact Import/Export</p>
          </div>
          <Database className="w-12 h-12 text-orange-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Contacts</div>
            <div className="text-2xl font-bold text-orange-400">{metrics.totalContacts.toLocaleString()}</div>
            <div className="text-xs text-gray-500 mt-2">Active & tracked</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Pipeline Value</div>
            <div className="text-2xl font-bold text-green-400">
              ${(metrics.totalPipeline / 1000000).toFixed(1)}M
            </div>
            <div className="text-xs text-gray-500 mt-2">{metrics.activeDeals} active deals</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Conversion Rate</div>
            <div className="text-2xl font-bold text-blue-400">{metrics.conversionRate}%</div>
            <div className="text-xs text-gray-500 mt-2">Lead to customer</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Deal Size</div>
            <div className="text-2xl font-bold text-purple-400">
              ${(metrics.avgDealSize / 1000).toFixed(0)}K
            </div>
            <div className="text-xs text-gray-500 mt-2">Per transaction</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['contacts', 'deals', 'campaigns', 'metrics'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-orange-400 border-b-2 border-orange-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'contacts' && 'Contacts'}
              {tab === 'deals' && 'Deals Pipeline'}
              {tab === 'campaigns' && 'Email Campaigns'}
              {tab === 'metrics' && 'Metrics'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'contacts' && (
            <div>
              <div className="flex justify-between items-center mb-4">
                <h3 className="text-lg font-semibold text-white">Contact Database</h3>
                <div className="flex gap-2">
                  <button className="flex items-center gap-2 bg-green-500/20 text-green-400 px-3 py-2 rounded text-sm font-semibold hover:bg-green-500/30">
                    <Download className="w-4 h-4" /> Import
                  </button>
                  <button className="flex items-center gap-2 bg-blue-500/20 text-blue-400 px-3 py-2 rounded text-sm font-semibold hover:bg-blue-500/30">
                    <Mail className="w-4 h-4" /> Export
                  </button>
                </div>
              </div>
              <div className="space-y-3">
                {contacts.map((contact) => (
                  <div key={contact.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{contact.name}</h4>
                        <p className="text-sm text-gray-400">{contact.company}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          contact.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : contact.status === 'lead'
                              ? 'bg-yellow-500/20 text-yellow-400'
                              : 'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {contact.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm mb-3">
                      <div>
                        <div className="text-gray-400">Email</div>
                        <div className="text-white font-semibold text-xs">{contact.email}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Phone</div>
                        <div className="text-white font-semibold text-xs">{contact.phone}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Last Contact</div>
                        <div className="text-white font-semibold">
                          {Math.round((Date.now() - contact.lastContact) / 86400000)}d ago
                        </div>
                      </div>
                      <div className="flex gap-1 flex-wrap">
                        {contact.tags.map((tag) => (
                          <span key={tag} className="bg-purple-500/20 text-purple-400 px-2 py-1 rounded text-xs">
                            {tag}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'deals' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Sales Pipeline</h3>
              <div className="space-y-4">
                {deals.map((deal) => (
                  <div key={deal.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{deal.name}</h4>
                        <p className="text-sm text-gray-400">Owner: {deal.owner}</p>
                      </div>
                      <div className={`px-3 py-1 rounded-full text-xs font-semibold ${stageColors[deal.stage]}`}>
                        {deal.stage.replace('-', ' ').toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 mb-3 text-sm">
                      <div>
                        <div className="text-gray-400">Deal Value</div>
                        <div className="text-white font-semibold">${(deal.value / 1000).toFixed(0)}K</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Win Probability</div>
                        <div className="text-white font-semibold">{deal.probability}%</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Days Open</div>
                        <div className="text-white font-semibold">
                          {Math.round((Date.now() - deal.createdDate) / 86400000)}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Expected Close</div>
                        <div className="text-white font-semibold">
                          {Math.round((deal.expectedClose - Date.now()) / 86400000)}d
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Weighted Value</div>
                        <div className="text-green-400 font-semibold">
                          ${((deal.value * deal.probability) / 100000).toFixed(0)}K
                        </div>
                      </div>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className={`h-2 rounded-full transition-all ${
                          deal.probability > 75
                            ? 'bg-gradient-to-r from-green-500 to-green-400'
                            : deal.probability > 50
                              ? 'bg-gradient-to-r from-blue-500 to-blue-400'
                              : 'bg-gradient-to-r from-yellow-500 to-yellow-400'
                        }`}
                        style={{ width: `${deal.probability}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'campaigns' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Email Campaigns</h3>
              <div className="space-y-4">
                {emailCampaigns.map((campaign) => {
                  const openRate = ((campaign.opened / campaign.sent) * 100).toFixed(1);
                  const clickRate = ((campaign.clicked / campaign.opened) * 100).toFixed(1);
                  return (
                    <div key={campaign.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                      <div className="flex items-start justify-between mb-4">
                        <div>
                          <h4 className="text-white font-semibold">{campaign.name}</h4>
                          <p className="text-sm text-gray-400">{campaign.subject}</p>
                        </div>
                        <div
                          className={`px-3 py-1 rounded-full text-xs font-semibold ${
                            campaign.status === 'sending'
                              ? 'bg-blue-500/20 text-blue-400'
                              : 'bg-green-500/20 text-green-400'
                          }`}
                        >
                          {campaign.status.toUpperCase()}
                        </div>
                      </div>
                      <div className="grid grid-cols-5 gap-4 text-sm">
                        <div>
                          <div className="text-gray-400">Sent</div>
                          <div className="text-white font-semibold">{campaign.sent.toLocaleString()}</div>
                        </div>
                        <div>
                          <div className="text-gray-400">Open Rate</div>
                          <div className="text-blue-400 font-semibold">{openRate}%</div>
                        </div>
                        <div>
                          <div className="text-gray-400">Click Rate</div>
                          <div className="text-green-400 font-semibold">{clickRate || '0'}%</div>
                        </div>
                        <div>
                          <div className="text-gray-400">Bounced</div>
                          <div className="text-red-400 font-semibold">{campaign.bounced}</div>
                        </div>
                        <div>
                          <div className="text-gray-400">Conversions</div>
                          <div className="text-purple-400 font-semibold">{Math.floor(campaign.clicked * 0.25)}</div>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {activeTab === 'metrics' && (
            <div className="grid grid-cols-3 gap-6">
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Contact Quality</h4>
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span className="text-gray-400">Active Contacts</span>
                      <span className="text-white font-semibold">65%</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div className="bg-green-500 h-2 rounded-full" style={{ width: '65%' }} />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span className="text-gray-400">Leads</span>
                      <span className="text-white font-semibold">28%</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div className="bg-yellow-500 h-2 rounded-full" style={{ width: '28%' }} />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span className="text-gray-400">Inactive</span>
                      <span className="text-white font-semibold">7%</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div className="bg-gray-500 h-2 rounded-full" style={{ width: '7%' }} />
                    </div>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Sales Velocity</h4>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Avg Sales Age</span>
                    <span className="text-white font-semibold">{metrics.avgSalesAge} days</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Deals This Month</span>
                    <span className="text-green-400 font-semibold">7</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Closed Won</span>
                    <span className="text-green-400 font-semibold">$485K</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Revenue per Day</span>
                    <span className="text-blue-400 font-semibold">$15.6K</span>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Team Performance</h4>
                <div className="space-y-3 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Sarah Chen</span>
                    <span className="text-green-400 font-semibold">$450K</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Mike Torres</span>
                    <span className="text-green-400 font-semibold">$320K</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Jessica Park</span>
                    <span className="text-green-400 font-semibold">$195K</span>
                  </div>
                  <div className="flex justify-between font-semibold border-t border-[#2a2a35] pt-3 mt-3">
                    <span className="text-gray-300">Total</span>
                    <span className="text-blue-400">$965K</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default CRMDatabasePanel;
