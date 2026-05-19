/**
 * Documentation.tsx — GPU Swarm Dashboard documentation viewer.
 *
 * Features:
 * - Browse all documentation files
 * - Render markdown with syntax highlighting
 * - Search functionality
 * - Responsive dark theme
 */

import React, { useState, useMemo } from "react";
import MarkdownViewer from "./MarkdownViewer";

interface DocFile {
  id: string;
  name: string;
  title: string;
  path: string;
  description: string;
  category: "getting-started" | "deployment" | "ci-cd" | "reference" | "architecture" | "blockchain-connector";
}

const DOCUMENTATION_FILES: DocFile[] = [
  // Getting Started
  {
    id: "index",
    name: "DOCUMENTATION_INDEX.md",
    title: "Documentation Index",
    path: "/docs/DOCUMENTATION_INDEX.md",
    description: "Master index and navigation guide for all documentation",
    category: "getting-started",
  },
  {
    id: "quick-start",
    name: "docs/root/README.md",
    title: "Quick Start",
    path: "/docs/docs/root/README.md",
    description: "Quick start guide and project overview",
    category: "getting-started",
  },
  {
    id: "project-completion",
    name: "PROJECT_COMPLETION.md",
    title: "Project Completion",
    path: "/docs/PROJECT_COMPLETION.md",
    description: "Full project completion summary",
    category: "getting-started",
  },

  // Deployment
  {
    id: "deployment-checklist",
    name: "docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md",
    title: "Deployment Checklist",
    path: "/docs/docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md",
    description: "Step-by-step deployment guide with SSH, Docker, and K8s options",
    category: "deployment",
  },
  {
    id: "pre-deployment",
    name: "PRE_docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md",
    title: "Pre-Deployment Verification",
    path: "/docs/PRE_docs/runbooks/deployment/DEPLOYMENT_CHECKLIST.md",
    description: "Comprehensive verification checklist before deployment",
    category: "deployment",
  },
  {
    id: "deployment",
    name: "DEPLOYMENT.md",
    title: "Deployment Guide",
    path: "/docs/DEPLOYMENT.md",
    description: "Server setup and deployment procedures",
    category: "deployment",
  },

  // CI/CD
  {
    id: "secrets-setup",
    name: "SECRETS_SETUP.md",
    title: "GitHub Secrets Configuration",
    path: "/docs/SECRETS_SETUP.md",
    description: "Configure GitHub Actions secrets and deployment credentials",
    category: "ci-cd",
  },
  {
    id: "ci-cd",
    name: "CI_CD_GUIDE.md",
    title: "CI/CD Pipeline Guide",
    path: "/docs/CI_CD_GUIDE.md",
    description: "Complete CI/CD pipeline documentation",
    category: "ci-cd",
  },
  {
    id: "status",
    name: "STATUS_AND_NEXT_STEPS.md",
    title: "Status & Next Steps",
    path: "/docs/STATUS_AND_NEXT_STEPS.md",
    description: "Current status, capabilities, and recommended next steps",
    category: "ci-cd",
  },

  // Reference
  {
    id: "makefile",
    name: "MAKEFILE_REFERENCE.md",
    title: "Makefile Reference",
    path: "/docs/MAKEFILE_REFERENCE.md",
    description: "30+ Makefile targets for build automation",
    category: "reference",
  },
  {
    id: "docs-summary",
    name: "DOCS_SUMMARY.md",
    title: "Documentation Summary",
    path: "/docs/DOCS_SUMMARY.md",
    description: "Overview of all documentation",
    category: "reference",
  },

  // Architecture
  {
    id: "implementation",
    name: "docs/reports/IMPLEMENTATION_SUMMARY.md",
    title: "Implementation Summary",
    path: "/docs/docs/reports/IMPLEMENTATION_SUMMARY.md",
    description: "Architecture overview and component details",
    category: "architecture",
  },
  {
    id: "build-report",
    name: "BUILD_REPORT.md",
    title: "Build Report",
    path: "/docs/BUILD_REPORT.md",
    description: "Build metrics and performance analysis",
    category: "architecture",
  },

  // Blockchain Connector
  {
    id: "blockchain-connector-overview",
    name: "BLOCKCHAIN_CONNECTOR.md",
    title: "Blockchain Connector — Overview",
    path: "/docs/BLOCKCHAIN_CONNECTOR.md",
    description: "Enterprise multi-chain connector: architecture, GPU advantage, and supported networks",
    category: "blockchain-connector",
  },
  {
    id: "blockchain-connector-api",
    name: "BLOCKCHAIN_CONNECTOR_API.md",
    title: "Connector API Reference",
    path: "/docs/BLOCKCHAIN_CONNECTOR_API.md",
    description: "REST + WebSocket + gRPC endpoints, SDK methods, and type definitions",
    category: "blockchain-connector",
  },
  {
    id: "blockchain-connector-benchmarks",
    name: "BLOCKCHAIN_CONNECTOR_BENCHMARKS.md",
    title: "Benchmark & Test Profiles",
    path: "/docs/BLOCKCHAIN_CONNECTOR_BENCHMARKS.md",
    description: "8 test profiles, GPU crypto benchmarks, and scoring methodology",
    category: "blockchain-connector",
  },
];

const CATEGORY_COLORS: Record<DocFile["category"], string> = {
  "getting-started": "text-green-400",
  deployment: "text-blue-400",
  "ci-cd": "text-purple-400",
  reference: "text-yellow-400",
  architecture: "text-orange-400",
  "blockchain-connector": "text-red-400",
};

const Documentation: React.FC = () => {
  const [selectedDoc, setSelectedDoc] = useState<DocFile>(DOCUMENTATION_FILES[0]!);
  const [searchQuery, setSearchQuery] = useState("");

  const filteredDocs = useMemo(() => {
    if (!searchQuery.trim()) return DOCUMENTATION_FILES;

    const query = searchQuery.toLowerCase();
    return DOCUMENTATION_FILES.filter(
      (doc) =>
        doc.title.toLowerCase().includes(query) ||
        doc.description.toLowerCase().includes(query) ||
        doc.name.toLowerCase().includes(query)
    );
  }, [searchQuery]);

  const categorizedDocs = useMemo(() => {
    const categories = {
      "getting-started": [] as DocFile[],
      deployment: [] as DocFile[],
      "ci-cd": [] as DocFile[],
      reference: [] as DocFile[],
      architecture: [] as DocFile[],
      "blockchain-connector": [] as DocFile[],
    };

    filteredDocs.forEach((doc) => {
      categories[doc.category].push(doc);
    });

    return categories;
  }, [filteredDocs]);

  return (
    <div className="flex h-full bg-background-primary text-text-primary">
      {/* Sidebar */}
      <div className="w-80 border-r border-border-default overflow-y-auto flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-border-default flex-shrink-0">
          <h2 className="text-lg font-bold mb-3 flex items-center gap-2">
            <span className="text-2xl">📚</span>
            Documentation
          </h2>

          {/* Search */}
          <input
            type="text"
            placeholder="Search docs..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full px-3 py-2 bg-background-secondary border border-border-default 
              rounded-lg text-text-primary placeholder-text-secondary text-sm
              focus:outline-none focus:border-accent-primary"
          />
        </div>

        {/* Doc List */}
        <div className="flex-1 overflow-y-auto">
          {Object.entries(categorizedDocs).map(([category, docs]) => {
            if (docs.length === 0) return null;

            return (
              <div key={category} className="py-2">
                <div className={`px-4 py-2 text-xs font-semibold uppercase tracking-wider ${CATEGORY_COLORS[category as DocFile["category"]]}`}>
                  {category === "getting-started" && "🚀 Getting Started"}
                  {category === "deployment" && "📦 Deployment"}
                  {category === "ci-cd" && "⚙️ CI/CD"}
                  {category === "reference" && "📖 Reference"}
                  {category === "architecture" && "🏗️ Architecture"}
                  {category === "blockchain-connector" && "⛓ Blockchain Connector"}
                </div>

                {docs.map((doc) => (
                  <button
                    key={doc.id}
                    onClick={() => setSelectedDoc(doc)}
                    className={`w-full text-left px-4 py-3 transition-colors
                      ${
                        selectedDoc.id === doc.id
                          ? "bg-accent-primary/20 border-l-2 border-accent-primary"
                          : "hover:bg-background-secondary border-l-2 border-transparent"
                      }`}
                  >
                    <div className="font-medium text-sm truncate">{doc.title}</div>
                    <div className="text-xs text-text-secondary truncate">
                      {doc.description}
                    </div>
                  </button>
                ))}
              </div>
            );
          })}
        </div>
      </div>

      {/* Main Viewer */}
      <div className="flex-1 overflow-y-auto p-8">
        <MarkdownViewer
          filePath={selectedDoc.path}
          title={selectedDoc.title}
        />
      </div>
    </div>
  );
};

export default Documentation;
