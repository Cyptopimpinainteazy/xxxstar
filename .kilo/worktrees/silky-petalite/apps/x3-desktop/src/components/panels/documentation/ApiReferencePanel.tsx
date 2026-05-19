import React, { useState } from "react";
import { Code, Copy, Search, ChevronDown, BookOpen, Package, Zap } from "lucide-react";
import clsx from "clsx";

interface ApiEndpoint {
  id: string;
  method: "GET" | "POST" | "PUT" | "DELETE";
  path: string;
  description: string;
  category: string;
  params: { name: string; type: string; required: boolean }[];
  example: string;
  response: string;
}

const API_ENDPOINTS: ApiEndpoint[] = [
  {
    id: "1",
    method: "GET",
    path: "/api/validators",
    description: "List all active validators",
    category: "Validators",
    params: [
      { name: "limit", type: "number", required: false },
      { name: "offset", type: "number", required: false },
    ],
    example: `curl -X GET "https://api.x3chain.io/api/validators?limit=10"`,
    response: `{ validators: [...], total: 142, page: 1 }`,
  },
  {
    id: "2",
    method: "POST",
    path: "/api/stake",
    description: "Create a new staking position",
    category: "Staking",
    params: [
      { name: "validatorId", type: "string", required: true },
      { name: "amount", type: "number", required: true },
    ],
    example: `curl -X POST "https://api.x3chain.io/api/stake" -d '{"validatorId":"v1","amount":100}'`,
    response: `{ stakeId: "s123", status: "pending", tx: "0xabc..." }`,
  },
  {
    id: "3",
    method: "GET",
    path: "/api/prices/{token}",
    description: "Get token price data",
    category: "Pricing",
    params: [
      { name: "token", type: "string", required: true },
      { name: "period", type: "string", required: false },
    ],
    example: `curl -X GET "https://api.x3chain.io/api/prices/X3?period=24h"`,
    response: `{ token: "X3", price: 1.25, change: 5.2, volume: 2500000 }`,
  },
];

export default function ApiReferencePanel() {
  const [selectedEndpoint, setSelectedEndpoint] = useState<ApiEndpoint | null>(API_ENDPOINTS[0]);
  const [searchQuery, setSearchQuery] = useState("");
  const [copiedCode, setCopiedCode] = useState(false);

  const filteredEndpoints = API_ENDPOINTS.filter(
    (ep) =>
      ep.path.toLowerCase().includes(searchQuery.toLowerCase()) ||
      ep.description.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const categories = [...new Set(API_ENDPOINTS.map((ep) => ep.category))];

  const handleCopyCode = () => {
    if (selectedEndpoint) {
      navigator.clipboard.writeText(selectedEndpoint.example);
      setCopiedCode(true);
      setTimeout(() => setCopiedCode(false), 2000);
    }
  };

  const getMethodColor = (method: string) => {
    switch (method) {
      case "GET":
        return "bg-blue-500/20 text-blue-300";
      case "POST":
        return "bg-green-500/20 text-green-300";
      case "PUT":
        return "bg-yellow-500/20 text-yellow-300";
      case "DELETE":
        return "bg-red-500/20 text-red-300";
      default:
        return "bg-gray-500/20 text-gray-300";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">API Reference</h2>

      <div className="flex gap-4 h-full min-h-0">
        {/* Left: Endpoints List */}
        <div className="w-80 flex flex-col border-r border-[#2a2a35]">
          <div className="mb-4 flex-shrink-0">
            <div className="relative">
              <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
              <input
                type="text"
                placeholder="Search endpoints..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded-lg pl-9 pr-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto pr-2">
            {filteredEndpoints.length === 0 ? (
              <div className="text-center text-gray-500 h-full flex items-center justify-center">
                No endpoints found
              </div>
            ) : (
              <div className="space-y-2">
                {filteredEndpoints.map((endpoint) => (
                  <button
                    key={endpoint.id}
                    onClick={() => setSelectedEndpoint(endpoint)}
                    className={clsx(
                      "w-full text-left px-3 py-2 rounded-lg transition border",
                      selectedEndpoint?.id === endpoint.id
                        ? "bg-blue-600/20 border-blue-400"
                        : "bg-[#15151b] border-[#2a2a35] hover:border-[#3a3a45]"
                    )}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span className={clsx("px-1.5 py-0.5 rounded text-xs font-bold", getMethodColor(endpoint.method))}>
                        {endpoint.method}
                      </span>
                      <span className="text-xs font-semibold font-mono flex-1 truncate">{endpoint.path}</span>
                    </div>
                    <div className="text-xs text-gray-500">{endpoint.description}</div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right: Endpoint Details */}
        {selectedEndpoint && (
          <div className="flex-1 overflow-y-auto pl-4">
            <div className="space-y-6">
              {/* Header */}
              <div>
                <div className="flex items-center gap-3 mb-2">
                  <span className={clsx("px-2 py-1 rounded-lg font-bold text-sm", getMethodColor(selectedEndpoint.method))}>
                    {selectedEndpoint.method}
                  </span>
                  <span className="font-mono font-semibold text-lg">{selectedEndpoint.path}</span>
                </div>
                <p className="text-gray-400">{selectedEndpoint.description}</p>
              </div>

              {/* Category Tag */}
              <div className="flex items-center gap-2">
                <Package size={16} className="text-gray-500" />
                <span className="text-xs font-semibold text-gray-400">Category: {selectedEndpoint.category}</span>
              </div>

              {/* Parameters */}
              {selectedEndpoint.params.length > 0 && (
                <div>
                  <h4 className="text-sm font-bold mb-3 flex items-center gap-2">
                    <Zap size={16} /> Parameters
                  </h4>
                  <div className="space-y-2 bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
                    {selectedEndpoint.params.map((param) => (
                      <div key={param.name} className="grid grid-cols-3 gap-2 text-sm">
                        <div className="font-mono font-semibold">{param.name}</div>
                        <div className="text-gray-500">{param.type}</div>
                        <div className={clsx("text-xs", param.required ? "text-red-400" : "text-gray-500")}>
                          {param.required ? "REQUIRED" : "optional"}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Example Code */}
              <div>
                <h4 className="text-sm font-bold mb-3 flex items-center gap-2">
                  <Code size={16} /> Example Request
                </h4>
                <div className="relative bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
                  <pre className="text-xs font-mono text-gray-300 overflow-x-auto whitespace-pre-wrap break-words">
                    {selectedEndpoint.example}
                  </pre>
                  <button
                    onClick={handleCopyCode}
                    className="absolute top-3 right-3 p-1 hover:bg-[#2a2a35] rounded transition"
                  >
                    <Copy size={14} className={copiedCode ? "text-green-400" : "text-gray-400"} />
                  </button>
                </div>
              </div>

              {/* Response */}
              <div>
                <h4 className="text-sm font-bold mb-3">Response</h4>
                <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
                  <pre className="text-xs font-mono text-gray-300 overflow-x-auto whitespace-pre-wrap break-words">
                    {selectedEndpoint.response}
                  </pre>
                </div>
              </div>

              {/* Learn More */}
              <div className="bg-blue-500/10 border border-blue-500/20 p-3 rounded-lg flex items-start gap-2 text-sm">
                <BookOpen size={16} className="text-blue-400 flex-shrink-0 mt-0.5" />
                <span className="text-gray-300">
                  For more details, visit{" "}
                  <span className="text-blue-400 font-semibold cursor-pointer hover:underline">
                    docs.x3chain.io/api
                  </span>
                </span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
