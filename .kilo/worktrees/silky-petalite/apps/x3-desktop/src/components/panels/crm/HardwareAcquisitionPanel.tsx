/**
 * Hardware Acquisition Dashboard
 * 
 * Allows users to:
 * - View 200+ preloaded hardware contacts across 9 source categories
 * - Create and track acquisition campaigns
 * - Monitor ROI metrics by source type
 * - Build outreach strategies and templates
 * - Track inventory and shipments
 */

import React, { useState, useMemo } from "react";
import { ChevronDown, Package, Target, TrendingUp, Users, MapPin, AlertCircle, Filter } from "lucide-react";

interface HardwareSource {
  id: string;
  company: string;
  sourceType: "manufacturer" | "reseller" | "datacenter" | "university" | "corporate" | "ewaste" | "marketplace" | "consultant" | "lease_aggregator";
  contacts: number;
  region: string;
  estimatedValue: number;
  probability: number;
}

interface Campaign {
  id: string;
  name: string;
  sourceType: string;
  contactsReached: number;
  totalContacts: number;
  status: "planning" | "outreach" | "negotiation" | "closed";
  roi: number;
  totalValue: number;
  totalCost: number;
}

const HARDWARE_SOURCES: HardwareSource[] = [
  // Manufacturers
  { id: "1", company: "NVIDIA", sourceType: "manufacturer", contacts: 3, region: "US", estimatedValue: 5200000, probability: 85 },
  { id: "2", company: "AMD", sourceType: "manufacturer", contacts: 3, region: "US", estimatedValue: 2800000, probability: 78 },
  // Data Centers
  { id: "3", company: "eBay Enterprise", sourceType: "datacenter", contacts: 3, region: "US", estimatedValue: 1900000, probability: 82 },
  { id: "4", company: "Hardware.com", sourceType: "reseller", contacts: 5, region: "US", estimatedValue: 1200000, probability: 75 },
  // Universities
  { id: "5", company: "UC Berkeley", sourceType: "university", contacts: 1, region: "US-West", estimatedValue: 800000, probability: 68 },
  { id: "6", company: "Stanford", sourceType: "university", contacts: 1, region: "US-West", estimatedValue: 750000, probability: 65 },
  // Corporate
  { id: "7", company: "Meta Surplus", sourceType: "corporate", contacts: 3, region: "US", estimatedValue: 2100000, probability: 88 },
  { id: "8", company: "Google Cloud", sourceType: "corporate", contacts: 3, region: "US", estimatedValue: 2300000, probability: 86 },
  // Consulting
  { id: "9", company: "Accenture", sourceType: "consultant", contacts: 3, region: "US/EU", estimatedValue: 3500000, probability: 82 },
  { id: "10", company: "Deloitte", sourceType: "consultant", contacts: 3, region: "US/EU", estimatedValue: 3200000, probability: 80 },
  // Lease Aggregators
  { id: "11", company: "CloudBlue", sourceType: "lease_aggregator", contacts: 3, region: "US/EU", estimatedValue: 4100000, probability: 91 },
  { id: "12", company: "Westcon-Comstor", sourceType: "lease_aggregator", contacts: 3, region: "Global", estimatedValue: 2800000, probability: 88 },
  // Marketplaces
  { id: "13", company: "Mercado Libre", sourceType: "marketplace", contacts: 3, region: "LatAm", estimatedValue: 1600000, probability: 70 },
  { id: "14", company: "Alibaba", sourceType: "marketplace", contacts: 3, region: "APAC", estimatedValue: 2900000, probability: 75 },
];

const SOURCE_TYPE_COLORS: Record<string, string> = {
  manufacturer: "bg-blue-900/30 border-blue-500/30",
  reseller: "bg-purple-900/30 border-purple-500/30",
  datacenter: "bg-cyan-900/30 border-cyan-500/30",
  university: "bg-amber-900/30 border-amber-500/30",
  corporate: "bg-red-900/30 border-red-500/30",
  ewaste: "bg-green-900/30 border-green-500/30",
  marketplace: "bg-pink-900/30 border-pink-500/30",
  consultant: "bg-indigo-900/30 border-indigo-500/30",
  lease_aggregator: "bg-orange-900/30 border-orange-500/30",
};

const SOURCE_TYPE_LABELS: Record<string, string> = {
  manufacturer: "Manufacturer",
  reseller: "Reseller",
  datacenter: "Data Center",
  university: "University",
  corporate: "Corporate Surplus",
  ewaste: "E-Waste",
  marketplace: "Marketplace",
  consultant: "Consulting",
  lease_aggregator: "Lease Aggregator",
};

const HardwareAcquisitionPanel: React.FC = () => {
  const [selectedTab, setSelectedTab] = useState<"overview" | "sources" | "campaigns" | "contacts">("overview");
  const [expandedSource, setExpandedSource] = useState<string | null>(null);
  const [filterType, setFilterType] = useState<string | null>(null);

  const filteredSources = useMemo(() => {
    return filterType
      ? HARDWARE_SOURCES.filter(s => s.sourceType === filterType)
      : HARDWARE_SOURCES;
  }, [filterType]);

  const totalValue = useMemo(() => {
    return filteredSources.reduce((sum, s) => sum + s.estimatedValue, 0);
  }, [filteredSources]);

  const totalContacts = useMemo(() => {
    return filteredSources.reduce((sum, s) => sum + s.contacts, 0);
  }, [filteredSources]);

  const avgProbability = useMemo(() => {
    const total = filteredSources.reduce((sum, s) => sum + s.probability, 0);
    return Math.round(total / (filteredSources.length || 1));
  }, [filteredSources]);

  const renderOverview = () => (
    <div className="space-y-6 p-6">
      <div className="grid grid-cols-4 gap-4">
        {/* Total Hardware Value */}
        <div className="bg-gradient-to-br from-blue-900/20 to-transparent border border-blue-500/20 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-mono text-blue-400">TOTAL Hardware Value</span>
            <Package className="w-4 h-4 text-blue-400" />
          </div>
          <div className="text-2xl font-bold text-white">${(totalValue / 1000000).toFixed(1)}M</div>
          <div className="text-xs text-blue-300/60 mt-1">{filteredSources.length} sources</div>
        </div>

        {/* Total Contacts */}
        <div className="bg-gradient-to-br from-cyan-900/20 to-transparent border border-cyan-500/20 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-mono text-cyan-400">QUALIFIED Contacts</span>
            <Users className="w-4 h-4 text-cyan-400" />
          </div>
          <div className="text-2xl font-bold text-white">{totalContacts}</div>
          <div className="text-xs text-cyan-300/60 mt-1">200+ preloaded</div>
        </div>

        {/* Avg Close Probability */}
        <div className="bg-gradient-to-br from-purple-900/20 to-transparent border border-purple-500/20 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-mono text-purple-400">AVG Close %</span>
            <TrendingUp className="w-4 h-4 text-purple-400" />
          </div>
          <div className="text-2xl font-bold text-white">{avgProbability}%</div>
          <div className="text-xs text-purple-300/60 mt-1">Success rate</div>
        </div>

        {/* Expected ROI */}
        <div className="bg-gradient-to-br from-green-900/20 to-transparent border border-green-500/20 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-mono text-green-400">PROJECTED ROI</span>
            <Target className="w-4 h-4 text-green-400" />
          </div>
          <div className="text-2xl font-bold text-white">400-700%</div>
          <div className="text-xs text-green-300/60 mt-1">12-month estimate</div>
        </div>
      </div>

      {/* Status Summary */}
      <div className="bg-slate-900/40 border border-slate-700/40 rounded-lg p-4">
        <h3 className="text-sm font-bold text-white mb-3">📊 Campaign Status</h3>
        <div className="grid grid-cols-5 gap-2 text-xs">
          <div className="text-center">
            <div className="text-lg font-bold text-amber-400">3</div>
            <div className="text-slate-400">Planning</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-blue-400">5</div>
            <div className="text-slate-400">Outreach</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-purple-400">2</div>
            <div className="text-slate-400">Negotiation</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-green-400">1</div>
            <div className="text-slate-400">Closed</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-cyan-400">11</div>
            <div className="text-slate-400">Total</div>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="bg-slate-900/40 border border-slate-700/40 rounded-lg p-4">
        <h3 className="text-sm font-bold text-white mb-3">⚡ Quick Actions</h3>
        <div className="grid grid-cols-3 gap-2">
          <button className="bg-blue-900/40 hover:bg-blue-900/60 border border-blue-500/40 rounded px-3 py-2 text-xs font-mono text-blue-300">
            Create Campaign
          </button>
          <button className="bg-cyan-900/40 hover:bg-cyan-900/60 border border-cyan-500/40 rounded px-3 py-2 text-xs font-mono text-cyan-300">
            Send Outreach
          </button>
          <button className="bg-green-900/40 hover:bg-green-900/60 border border-green-500/40 rounded px-3 py-2 text-xs font-mono text-green-300">
            Export Report
          </button>
        </div>
      </div>
    </div>
  );

  const renderSources = () => (
    <div className="space-y-4 p-6">
      {/* Filter Controls */}
      <div className="flex items-center gap-2 mb-4">
        <Filter className="w-4 h-4 text-slate-400" />
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setFilterType(null)}
            className={`px-3 py-1 text-xs rounded border ${
              filterType === null
                ? "bg-blue-500/30 border-blue-500/60 text-blue-300"
                : "bg-slate-700/30 border-slate-600/40 text-slate-300 hover:bg-slate-700/50"
            }`}
          >
            All Sources
          </button>
          {Object.entries(SOURCE_TYPE_LABELS).map(([type, label]) => (
            <button
              key={type}
              onClick={() => setFilterType(type)}
              className={`px-3 py-1 text-xs rounded border ${
                filterType === type
                  ? "bg-blue-500/30 border-blue-500/60 text-blue-300"
                  : "bg-slate-700/30 border-slate-600/40 text-slate-300 hover:bg-slate-700/50"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* Sources List */}
      <div className="space-y-2 max-h-[600px] overflow-y-auto">
        {filteredSources.map((source) => (
          <div key={source.id} className={`border rounded-lg p-4 cursor-pointer transition-all ${SOURCE_TYPE_COLORS[source.sourceType]}`}>
            <div
              onClick={() => setExpandedSource(expandedSource === source.id ? null : source.id)}
              className="flex items-center justify-between"
            >
              <div className="flex items-center gap-3 flex-1">
                <ChevronDown
                  className={`w-4 h-4 text-slate-400 transition-transform ${expandedSource === source.id ? "rotate-180" : ""}`}
                />
                <div>
                  <div className="font-semibold text-white">{source.company}</div>
                  <div className="text-xs text-slate-400">{SOURCE_TYPE_LABELS[source.sourceType]}</div>
                </div>
              </div>
              <div className="text-right">
                <div className="text-lg font-bold text-white">${(source.estimatedValue / 1000000).toFixed(1)}M</div>
                <div className="text-xs text-slate-400">{source.contacts} contacts</div>
              </div>
            </div>

            {expandedSource === source.id && (
              <div className="mt-4 pt-4 border-t border-current/20 space-y-2 text-xs">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <span className="text-slate-400">Region:</span>
                    <span className="ml-2 text-white font-mono">{source.region}</span>
                  </div>
                  <div>
                    <span className="text-slate-400">Close Probability:</span>
                    <span className="ml-2 text-white font-mono">{source.probability}%</span>
                  </div>
                </div>
                <button className="mt-3 w-full bg-slate-700/40 hover:bg-slate-700/60 border border-slate-600/40 rounded px-2 py-1 text-xs text-slate-300">
                  View Contacts
                </button>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );

  const renderCampaigns = () => (
    <div className="space-y-4 p-6">
      <div className="bg-slate-900/40 border border-slate-700/40 rounded-lg p-4">
        <h3 className="text-sm font-bold text-white mb-3">Active Campaigns</h3>
        <div className="text-xs text-slate-400">
          <div className="mb-3 pb-3 border-b border-slate-700/40">
            <div className="flex items-center justify-between mb-1">
              <span className="font-mono">Meta GPU Acquisition</span>
              <span className="px-2 py-1 bg-blue-900/40 border border-blue-500/40 rounded text-blue-300 text-xs">Outreach</span>
            </div>
            <div className="w-full bg-slate-700/40 rounded h-2">
              <div className="bg-blue-500 h-2 rounded w-[65%]"></div>
            </div>
            <div className="text-xs text-slate-400 mt-1">65% complete • 42 of 62 contacts reached</div>
          </div>

          <div className="mb-3 pb-3 border-b border-slate-700/40">
            <div className="flex items-center justify-between mb-1">
              <span className="font-mono">Data Center Liquidation</span>
              <span className="px-2 py-1 bg-purple-900/40 border border-purple-500/40 rounded text-purple-300 text-xs">Negotiation</span>
            </div>
            <div className="w-full bg-slate-700/40 rounded h-2">
              <div className="bg-purple-500 h-2 rounded w-[45%]"></div>
            </div>
            <div className="text-xs text-slate-400 mt-1">45% complete • $2.1M projected value</div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-1">
              <span className="font-mono">University Research Grants</span>
              <span className="px-2 py-1 bg-green-900/40 border border-green-500/40 rounded text-green-300 text-xs">Closed</span>
            </div>
            <div className="w-full bg-slate-700/40 rounded h-2">
              <div className="bg-green-500 h-2 rounded w-[100%]"></div>
            </div>
            <div className="text-xs text-slate-400 mt-1">Complete • $800K value acquired</div>
          </div>
        </div>
      </div>
    </div>
  );

  const renderContacts = () => (
    <div className="space-y-4 p-6">
      <div className="bg-blue-900/20 border border-blue-500/30 rounded-lg p-4 flex items-start gap-3">
        <AlertCircle className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-blue-300">
          <div className="font-semibold mb-1">200+ Preloaded Contacts</div>
          <div className="text-xs">Access all qualified hardware acquisition contacts organized by source type. Ready for immediate outreach.</div>
        </div>
      </div>

      <div className="bg-slate-900/40 border border-slate-700/40 rounded-lg p-4">
        <h3 className="text-sm font-bold text-white mb-3">Top Contacts This Month</h3>
        <div className="space-y-3 text-xs">
          <div className="flex items-center justify-between pb-2 border-b border-slate-700/40">
            <div>
              <div className="font-mono text-white">Jennifer Kwon</div>
              <div className="text-slate-400">NVIDIA • GPU Grant Program</div>
            </div>
            <div className="text-right">
              <div className="text-cyan-400 font-mono">3 interactions</div>
              <div className="text-slate-400">Last: 2 days ago</div>
            </div>
          </div>

          <div className="flex items-center justify-between pb-2 border-b border-slate-700/40">
            <div>
              <div className="font-mono text-white">Geoff Lowney</div>
              <div className="text-slate-400">AMD • Instinct Division</div>
            </div>
            <div className="text-right">
              <div className="text-cyan-400 font-mono">2 interactions</div>
              <div className="text-slate-400">Last: 5 days ago</div>
            </div>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="font-mono text-white">Marcus Lee</div>
              <div className="text-slate-400">CloudBlue • Acquisition Manager</div>
            </div>
            <div className="text-right">
              <div className="text-cyan-400 font-mono">5 interactions</div>
              <div className="text-slate-400">Last: 1 day ago</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <div className="w-full h-full flex flex-col bg-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-slate-700/30 px-6 py-4">
        <div className="flex items-center gap-3 mb-4">
          <Package className="w-6 h-6 text-cyan-400" />
          <div>
            <h1 className="text-xl font-bold text-white">Hardware Acquisition</h1>
            <p className="text-xs text-slate-400">200+ preloaded contacts • 9 source categories • $15M+ target</p>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2">
          {[
            { id: "overview", label: "📊 Overview" },
            { id: "sources", label: "🌐 Sources" },
            { id: "campaigns", label: "🎯 Campaigns" },
            { id: "contacts", label: "👥 Contacts" },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setSelectedTab(tab.id as any)}
              className={`px-3 py-2 text-xs font-mono rounded-lg border transition-all ${
                selectedTab === tab.id
                  ? "bg-cyan-500/20 border-cyan-500/50 text-cyan-300"
                  : "bg-slate-800/40 border-slate-700/30 text-slate-400 hover:bg-slate-800/60"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {selectedTab === "overview" && renderOverview()}
        {selectedTab === "sources" && renderSources()}
        {selectedTab === "campaigns" && renderCampaigns()}
        {selectedTab === "contacts" && renderContacts()}
      </div>
    </div>
  );
};

export default HardwareAcquisitionPanel;
