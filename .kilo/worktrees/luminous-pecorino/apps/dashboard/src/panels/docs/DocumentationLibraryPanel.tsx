import React, { useState } from 'react';
import { Book, Code, ExternalLink, Copy, ChevronRight } from 'lucide-react';

interface Documentation {
  id: string;
  title: string;
  category: string;
  description: string;
  codeExample: string;
  link: string;
}

export const DocumentationLibraryPanel: React.FC = () => {
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [docs] = useState<Documentation[]>([
    {
      id: '1',
      title: 'Getting Started with X3',
      category: 'guide',
      description: 'Learn the basics of the X3 blockchain platform',
      codeExample: '// Initialize X3 client\nconst client = new X3Client();',
      link: '/docs/getting-started',
    },
    {
      id: '2',
      title: 'Smart Contract Development',
      category: 'guide',
      description: 'Write and deploy smart contracts on X3',
      codeExample: 'contract MyContract { pub state: String }',
      link: '/docs/contracts',
    },
    {
      id: '3',
      title: 'API Reference',
      category: 'api',
      description: 'Complete API documentation for X3',
      codeExample: 'GET /api/v1/blocks?limit=10',
      link: '/docs/api',
    },
    {
      id: '4',
      title: 'Validator Setup',
      category: 'tutorial',
      description: 'Step-by-step guide to running a validator node',
      codeExample: '$ x3-validator start --config config.toml',
      link: '/docs/validator-setup',
    },
    {
      id: '5',
      title: 'Security Best Practices',
      category: 'guide',
      description: 'Secure your validator and smart contracts',
      codeExample: '// Always validate inputs\nif (!isValidInput(data)) return;',
      link: '/docs/security',
    },
    {
      id: '6',
      title: 'GraphQL Queries',
      category: 'api',
      description: 'Query blockchain data with GraphQL',
      codeExample: 'query { blocks(limit: 10) { height timestamp } }',
      link: '/docs/graphql',
    },
  ]);

  const categories = [
    { id: 'all', label: 'All Docs' },
    { id: 'guide', label: 'Guides' },
    { id: 'api', label: 'API' },
    { id: 'tutorial', label: 'Tutorials' },
  ];

  const filteredDocs = selectedCategory === 'all' ? docs : docs.filter((d) => d.category === selectedCategory);

  const handleCopyCode = (code: string) => {
    navigator.clipboard.writeText(code);
  };

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'guide':
        return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
      case 'api':
        return 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20';
      case 'tutorial':
        return 'bg-purple-500/10 text-purple-400 border-purple-500/20';
      default:
        return 'bg-gray-500/10 text-gray-400 border-gray-500/20';
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Documentation Library
            </h1>
            <p className="text-gray-400">Guides, API references, and tutorials</p>
          </div>
          <Book className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Search & Filter */}
        <div className="mb-6">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 mb-4">
            <input
              type="text"
              placeholder="Search documentation..."
              className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
            />
          </div>

          <div className="flex gap-2 flex-wrap">
            {categories.map((cat) => (
              <button
                key={cat.id}
                onClick={() => setSelectedCategory(cat.id)}
                className={`px-4 py-2 rounded-lg font-semibold transition ${
                  selectedCategory === cat.id
                    ? 'bg-cyan-600 text-white'
                    : 'bg-[#1a1a2e] border border-[#2a2a35] text-gray-400 hover:border-cyan-400'
                }`}
              >
                {cat.label}
              </button>
            ))}
          </div>
        </div>

        {/* Documentation Cards */}
        <div className="grid grid-cols-1 gap-4 mb-8">
          {filteredDocs.map((doc) => (
            <div
              key={doc.id}
              className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden hover:border-cyan-400/50 transition group"
            >
              <div className="p-6">
                {/* Header */}
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1">
                    <h3 className="text-white font-bold text-lg mb-1 group-hover:text-cyan-400 transition">
                      {doc.title}
                    </h3>
                    <p className="text-gray-400 text-sm">{doc.description}</p>
                  </div>
                  <span className={`text-xs px-2 py-1 rounded border whitespace-nowrap ml-4 ${getCategoryColor(doc.category)}`}>
                    {doc.category}
                  </span>
                </div>

                {/* Code Example */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4 mb-4">
                  <div className="flex items-center justify-between mb-2">
                    <p className="text-gray-400 text-xs font-semibold">Example</p>
                    <button
                      onClick={() => handleCopyCode(doc.codeExample)}
                      className="text-cyan-400 hover:text-cyan-300 transition flex items-center gap-1 text-xs"
                    >
                      <Copy className="w-3 h-3" /> Copy
                    </button>
                  </div>
                  <pre className="text-gray-300 font-mono text-sm overflow-x-auto">
                    <code>{doc.codeExample}</code>
                  </pre>
                </div>

                {/* Footer */}
                <div className="flex items-center justify-end">
                  <a
                    href={doc.link}
                    className="text-cyan-400 hover:text-cyan-300 font-semibold text-sm flex items-center gap-1 transition"
                  >
                    Read More
                    <ExternalLink className="w-4 h-4" />
                  </a>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Quick Links */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          <h2 className="text-white font-bold text-lg mb-4">Quick Links</h2>
          <div className="grid grid-cols-2 gap-4">
            {[
              { title: 'API Status', link: '/api/health' },
              { title: 'GitHub Repository', link: 'https://github.com/x3-chain' },
              { title: 'Community Discord', link: 'https://discord.gg/x3' },
              { title: 'Changelog', link: '/docs/changelog' },
            ].map((item, idx) => (
              <a
                key={idx}
                href={item.link}
                className="flex items-center justify-between p-3 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg hover:border-cyan-400/50 transition group"
              >
                <span className="text-gray-300 group-hover:text-white transition font-semibold">{item.title}</span>
                <ChevronRight className="w-4 h-4 text-gray-500 group-hover:text-cyan-400 transition" />
              </a>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default DocumentationLibraryPanel;
