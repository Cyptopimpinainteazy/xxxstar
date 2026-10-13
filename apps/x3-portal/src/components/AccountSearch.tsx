'use client';

import React, { useState } from 'react';

interface SearchResult {
  type: 'account' | 'transaction';
  id: string;
  data: any;
}

export default function AccountSearch() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;

    setLoading(true);

    // Mock search results - replace with actual API calls
    setTimeout(() => {
      const mockResults: SearchResult[] = [
        {
          type: 'account',
          id: query,
          data: {
            balance: '123.456 ATLAS',
            transactions: 42,
            vm: 'EVM'
          }
        },
        {
          type: 'transaction',
          id: '0x' + query.slice(0, 16) + '...',
          data: {
            from: query,
            to: '0x742d35Cc6634C0532925a3b844Bc454e4438f44f',
            value: '1.234 ETH',
            status: 'success'
          }
        }
      ];

      setResults(mockResults);
      setLoading(false);
    }, 1000);
  };

  return (
    <div className="max-w-4xl mx-auto">
      <div className="bg-white shadow rounded-lg">
        <div className="px-4 py-5 sm:p-6">
          <h3 className="text-lg leading-6 font-medium text-gray-900 mb-4">Search Atlas Sphere</h3>
          <form onSubmit={handleSearch} className="flex gap-4">
            <div className="flex-1">
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search by address, transaction hash, or block number..."
                className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
            <button
              type="submit"
              disabled={loading}
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
            >
              {loading ? 'Searching...' : 'Search'}
            </button>
          </form>

          {results.length > 0 && (
            <div className="mt-6">
              <h4 className="text-md font-medium text-gray-900 mb-4">Search Results</h4>
              <div className="space-y-4">
                {results.map((result, index) => (
                  <div key={index} className="border border-gray-200 rounded-lg p-4">
                    <div className="flex items-center justify-between">
                      <div>
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                          result.type === 'account' ? 'bg-green-100 text-green-800' : 'bg-blue-100 text-blue-800'
                        }`}>
                          {result.type === 'account' ? 'Account' : 'Transaction'}
                        </span>
                        <p className="mt-1 text-sm font-medium text-gray-900">{result.id}</p>
                      </div>
                      <button className="text-blue-600 hover:text-blue-500 text-sm">
                        View Details →
                      </button>
                    </div>

                    {result.type === 'account' && (
                      <div className="mt-3 grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <span className="text-gray-500">Balance:</span>
                          <span className="ml-2 font-medium">{result.data.balance}</span>
                        </div>
                        <div>
                          <span className="text-gray-500">Transactions:</span>
                          <span className="ml-2 font-medium">{result.data.transactions}</span>
                        </div>
                      </div>
                    )}

                    {result.type === 'transaction' && (
                      <div className="mt-3 space-y-2 text-sm">
                        <div>
                          <span className="text-gray-500">From:</span>
                          <span className="ml-2 font-mono">{result.data.from.slice(0, 20)}...</span>
                        </div>
                        <div>
                          <span className="text-gray-500">To:</span>
                          <span className="ml-2 font-mono">{result.data.to.slice(0, 20)}...</span>
                        </div>
                        <div>
                          <span className="text-gray-500">Value:</span>
                          <span className="ml-2 font-medium">{result.data.value}</span>
                        </div>
                        <div>
                          <span className="text-gray-500">Status:</span>
                          <span className={`ml-2 ${result.data.status === 'success' ? 'text-green-600' : 'text-red-600'}`}>
                            {result.data.status === 'success' ? '✓ Success' : '✗ Failed'}
                          </span>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {query && !loading && results.length === 0 && (
            <div className="mt-6 text-center text-gray-500">
              No results found for "{query}"
            </div>
          )}
        </div>
      </div>
    </div>
  );
}