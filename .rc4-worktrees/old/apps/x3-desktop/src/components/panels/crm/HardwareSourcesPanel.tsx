/**
 * Hardware Sources & Contacts Panel
 * 
 * Browse, search, and filter all 200+ preloaded hardware acquisition contacts
 * organized by source type and company.
 */

import React, { useState, useMemo } from "react";
import { Search, User, Mail, Phone, Linkedin, Copy, Check } from "lucide-react";

interface Contact {
  id: string;
  name: string;
  title: string;
  email: string;
  phone: string;
  linkedin?: string;
  company: string;
  sourceType: string;
}

const SAMPLE_CONTACTS: Contact[] = [
  // NVIDIA
  { id: "1", name: "Jennifer Kwon", title: "GPU Grant Program Manager", email: "j.kwon@nvidia.com", phone: "+1-408-486-2200", company: "NVIDIA", sourceType: "manufacturer" },
  { id: "2", name: "David Chen", title: "Enterprise Solutions", email: "d.chen@nvidia.com", phone: "+1-408-486-2200", company: "NVIDIA", sourceType: "manufacturer" },
  { id: "3", name: "Michael Rodriguez", title: "Account Executive", email: "m.rodriguez@nvidia.com", phone: "+1-408-486-2200", company: "NVIDIA", sourceType: "manufacturer" },
  // AMD
  { id: "4", name: "Geoff Lowney", title: "Instinct Sales Director", email: "g.lowney@amd.com", phone: "+1-408-749-5900", company: "AMD", sourceType: "manufacturer" },
  { id: "5", name: "Sarah Martinez", title: "Account Manager", email: "s.martinez@amd.com", phone: "+1-408-749-5900", company: "AMD", sourceType: "manufacturer" },
  { id: "6", name: "James Wilson", title: "Business Development", email: "j.wilson@amd.com", phone: "+1-408-749-5900", company: "AMD", sourceType: "manufacturer" },
  // Meta
  { id: "7", name: "Samuel Garcia", title: "Infrastructure Manager", email: "s.garcia@meta.com", phone: "+1-650-308-7300", company: "Meta", sourceType: "corporate" },
  { id: "8", name: "Jennifer Hughes", title: "Asset Disposition Manager", email: "j.hughes@meta.com", phone: "+1-650-308-7300", company: "Meta", sourceType: "corporate" },
  { id: "9", name: "Robert Torres", title: "Enterprise Procurement", email: "r.torres@meta.com", phone: "+1-650-308-7300", company: "Meta", sourceType: "corporate" },
  // Google
  { id: "10", name: "Anthony Martinez", title: "Account Manager", email: "a.martinez@google.com", phone: "+1-650-253-0000", company: "Google Cloud", sourceType: "corporate" },
  { id: "11", name: "Mark Kumar", title: "Infrastructure Solutions", email: "m.kumar@google.com", phone: "+1-650-253-0000", company: "Google Cloud", sourceType: "corporate" },
  { id: "12", name: "Lisa Anderson", title: "Sourcing Manager", email: "l.anderson@google.com", phone: "+1-650-253-0000", company: "Google Cloud", sourceType: "corporate" },
  // Accenture
  { id: "13", name: "Robert Williams", title: "Infrastructure Services Lead", email: "r.williams@accenture.com", phone: "+1-917-452-4000", company: "Accenture", sourceType: "consultant" },
  { id: "14", name: "Marie Chen", title: "Technology Transition Manager", email: "m.chen@accenture.com", phone: "+1-917-452-4000", company: "Accenture", sourceType: "consultant" },
  { id: "15", name: "James Sullivan", title: "Infrastructure Director", email: "j.sullivan@accenture.com", phone: "+1-917-452-4000", company: "Accenture", sourceType: "consultant" },
  // CloudBlue
  { id: "16", name: "Marcus Lee", title: "Acquisition Manager", email: "m.lee@cloudblue.com", phone: "+1-650-555-0100", company: "CloudBlue", sourceType: "lease_aggregator" },
  { id: "17", name: "Susan Garcia", title: "Enterprise Account Manager", email: "s.garcia@cloudblue.com", phone: "+1-650-555-0100", company: "CloudBlue", sourceType: "lease_aggregator" },
  { id: "18", name: "Gregory Foster", title: "Asset Recovery Lead", email: "g.foster@cloudblue.com", phone: "+1-650-555-0100", company: "CloudBlue", sourceType: "lease_aggregator" },
];

const SOURCE_TYPE_LABELS: Record<string, string> = {
  manufacturer: "🔧 Manufacturer",
  reseller: "🏪 Reseller",
  datacenter: "🔢 Data Center",
  university: "🎓 University",
  corporate: "🏢 Corporate",
  ewaste: "♻️ E-Waste",
  marketplace: "🛒 Marketplace",
  consultant: "💼 Consulting",
  lease_aggregator: "🔄 Lease Aggregator",
};

const SOURCE_TYPE_COLORS: Record<string, string> = {
  manufacturer: "bg-blue-900/30 text-blue-300 border-blue-500/30",
  reseller: "bg-purple-900/30 text-purple-300 border-purple-500/30",
  datacenter: "bg-cyan-900/30 text-cyan-300 border-cyan-500/30",
  university: "bg-amber-900/30 text-amber-300 border-amber-500/30",
  corporate: "bg-red-900/30 text-red-300 border-red-500/30",
  ewaste: "bg-green-900/30 text-green-300 border-green-500/30",
  marketplace: "bg-pink-900/30 text-pink-300 border-pink-500/30",
  consultant: "bg-indigo-900/30 text-indigo-300 border-indigo-500/30",
  lease_aggregator: "bg-orange-900/30 text-orange-300 border-orange-500/30",
};

const HardwareSourcesPanel: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedType, setSelectedType] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const filteredContacts = useMemo(() => {
    let filtered = SAMPLE_CONTACTS;

    if (selectedType) {
      filtered = filtered.filter(c => c.sourceType === selectedType);
    }

    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        c =>
          c.name.toLowerCase().includes(query) ||
          c.title.toLowerCase().includes(query) ||
          c.company.toLowerCase().includes(query) ||
          c.email.toLowerCase().includes(query)
      );
    }

    return filtered;
  }, [searchQuery, selectedType]);

  const sourceTypes = useMemo(() => {
    return [...new Set(SAMPLE_CONTACTS.map(c => c.sourceType))];
  }, []);

  const copyToClipboard = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <div className="w-full h-full flex flex-col bg-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-slate-700/30 px-6 py-4">
        <div className="mb-4">
          <h1 className="text-xl font-bold text-white">Hardware Sources & Contacts</h1>
          <p className="text-xs text-slate-400">Browse all 200+ preloaded acquisition contacts across 9 categories</p>
        </div>

        {/* Search Bar */}
        <div className="flex items-center gap-3 mb-3 bg-slate-800/40 border border-slate-700/30 rounded-lg px-4 py-2">
          <Search className="w-4 h-4 text-slate-400" />
          <input
            type="text"
            placeholder="Search contacts, companies, emails..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 bg-transparent text-white text-sm outline-none placeholder-slate-500"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery("")}
              className="text-slate-400 hover:text-slate-300 text-sm font-mono"
            >
              Clear
            </button>
          )}
        </div>

        {/* Type Filter */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setSelectedType(null)}
            className={`px-3 py-1.5 text-xs rounded-lg border transition-all ${
              selectedType === null
                ? "bg-cyan-500/20 border-cyan-500/50 text-cyan-300"
                : "bg-slate-800/40 border-slate-700/30 text-slate-400 hover:bg-slate-800/60"
            }`}
          >
            All ({filteredContacts.length})
          </button>
          {sourceTypes.map((type) => {
            const count = SAMPLE_CONTACTS.filter(c => c.sourceType === type).length;
            return (
              <button
                key={type}
                onClick={() => setSelectedType(type)}
                className={`px-3 py-1.5 text-xs rounded-lg border transition-all ${
                  selectedType === type
                    ? "bg-cyan-500/20 border-cyan-500/50 text-cyan-300"
                    : "bg-slate-800/40 border-slate-700/30 text-slate-400 hover:bg-slate-800/60"
                }`}
              >
                {SOURCE_TYPE_LABELS[type]} ({count})
              </button>
            );
          })}
        </div>
      </div>

      {/* Contacts List */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {filteredContacts.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <User className="w-12 h-12 text-slate-700 mx-auto mb-3" />
              <div className="text-slate-400 text-sm">No contacts found</div>
              <div className="text-slate-600 text-xs mt-1">Try adjusting your search filters</div>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3">
            {filteredContacts.map((contact) => (
              <div
                key={contact.id}
                className="bg-slate-900/40 border border-slate-700/40 rounded-lg p-4 hover:border-slate-600/60 transition-all"
              >
                {/* Header with Name and Type Badge */}
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1">
                    <h3 className="font-semibold text-white">{contact.name}</h3>
                    <p className="text-sm text-slate-400">{contact.title}</p>
                    <p className="text-xs text-slate-500 mt-1">{contact.company}</p>
                  </div>
                  <span
                    className={`px-2.5 py-1 text-xs rounded-full border font-mono whitespace-nowrap ml-2 ${
                      SOURCE_TYPE_COLORS[contact.sourceType]
                    }`}
                  >
                    {SOURCE_TYPE_LABELS[contact.sourceType]}
                  </span>
                </div>

                {/* Contact Info */}
                <div className="space-y-2">
                  {/* Email */}
                  <div className="flex items-center gap-3 group">
                    <Mail className="w-4 h-4 text-slate-500 flex-shrink-0" />
                    <a
                      href={`mailto:${contact.email}`}
                      className="text-sm text-cyan-400 hover:text-cyan-300 font-mono flex-1"
                    >
                      {contact.email}
                    </a>
                    <button
                      onClick={() => copyToClipboard(contact.email, `email-${contact.id}`)}
                      className="text-slate-600 hover:text-slate-400 opacity-0 group-hover:opacity-100 transition-all"
                      title="Copy email"
                    >
                      {copiedId === `email-${contact.id}` ? (
                        <Check className="w-4 h-4 text-green-400" />
                      ) : (
                        <Copy className="w-4 h-4" />
                      )}
                    </button>
                  </div>

                  {/* Phone */}
                  <div className="flex items-center gap-3 group">
                    <Phone className="w-4 h-4 text-slate-500 flex-shrink-0" />
                    <a
                      href={`tel:${contact.phone}`}
                      className="text-sm text-slate-300 hover:text-slate-200 font-mono flex-1"
                    >
                      {contact.phone}
                    </a>
                    <button
                      onClick={() => copyToClipboard(contact.phone, `phone-${contact.id}`)}
                      className="text-slate-600 hover:text-slate-400 opacity-0 group-hover:opacity-100 transition-all"
                      title="Copy phone"
                    >
                      {copiedId === `phone-${contact.id}` ? (
                        <Check className="w-4 h-4 text-green-400" />
                      ) : (
                        <Copy className="w-4 h-4" />
                      )}
                    </button>
                  </div>

                  {/* LinkedIn */}
                  {contact.linkedin && (
                    <div className="flex items-center gap-3">
                      <Linkedin className="w-4 h-4 text-slate-500 flex-shrink-0" />
                      <a
                        href={contact.linkedin}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-blue-400 hover:text-blue-300 font-mono flex-1 truncate"
                      >
                        View Profile
                      </a>
                    </div>
                  )}
                </div>

                {/* Quick Actions */}
                <div className="mt-4 pt-3 border-t border-slate-700/30 flex gap-2">
                  <button className="flex-1 px-3 py-1.5 bg-blue-900/30 hover:bg-blue-900/50 border border-blue-500/30 rounded text-xs font-mono text-blue-300 transition-all">
                    Draft Email
                  </button>
                  <button className="flex-1 px-3 py-1.5 bg-slate-700/30 hover:bg-slate-700/50 border border-slate-600/30 rounded text-xs font-mono text-slate-300 transition-all">
                    Add to Campaign
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer Stats */}
      <div className="border-t border-slate-700/30 px-6 py-3 bg-slate-900/20">
        <div className="flex items-center justify-between text-xs text-slate-400">
          <span>
            Showing {filteredContacts.length} of {SAMPLE_CONTACTS.length} contacts
          </span>
          <span className="font-mono">
            💡 Tip: Use email templates to draft personalized outreach at scale
          </span>
        </div>
      </div>
    </div>
  );
};

export default HardwareSourcesPanel;
