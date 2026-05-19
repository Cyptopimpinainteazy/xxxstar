import { useState } from 'react';
import { useDorksSearch } from '../../lib/x3/hooks/useDorksSearch';
import { Button, Card, Progress, Badge, Spinner } from '@radix-ui/themes';

const OBJECTIVES = [
  { value: 'find_investors', label: 'Find Investors' },
  { value: 'find_grants', label: 'Find Grants' },
  { value: 'market_research', label: 'Market Research' },
  { value: 'partnership_search', label: 'Partnership Search' },
];

const SECTORS = [
  'AI/ML', 'SaaS', 'FinTech', 'ClimaTech', 'Web3', 'Healthcare',
  'E-commerce', 'B2B', 'B2C', 'DeepTech'
];

export function DorksSearchPanel() {
  const { 
    generatedQueries, 
    results, 
    loading, 
    error,
    autogenerateSearachQueries,
    executeDorksSearch,
    bulkImportResults,
  } = useDorksSearch();

  const [objective, setObjective] = useState('find_investors');
  const [sectors, setSectors] = useState<string[]>(['AI/ML']);
  const [keywords, setKeywords] = useState('');
  const [selectedResults, setSelectedResults] = useState<Set<string>>(new Set());
  const [searchExecuted, setSearchExecuted] = useState(false);

  const handleGenerateQueries = async () => {
    const keywordList = keywords.split(',').map(k => k.trim()).filter(k => k);
    if (keywordList.length === 0) return;
    
    await autogenerateSearachQueries(objective, keywordList);
  };

  const handleExecuteQuery = async (query: string) => {
    await executeDorksSearch(query);
    setSearchExecuted(true);
  };

  const handleImportSelected = async () => {
    const toImport = results.filter(r => selectedResults.has(r.url));
    if (toImport.length > 0) {
      await bulkImportResults(toImport);
      setSelectedResults(new Set());
    }
  };

  return (
    <div className="space-y-6">
      {/* Search Configuration */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Search Configuration</h3>
        
        <div className="grid grid-cols-2 gap-4 mb-4">
          <div>
            <label className="block text-sm font-medium mb-2">Objective</label>
            <select value={objective} onChange={(e) => setObjective(e.target.value)} className="w-full border rounded px-2 py-1">
              {OBJECTIVES.map(opt => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Focus Sectors</label>
            <select 
              multiple 
              value={sectors}
              onChange={(e) => setSectors(Array.from(e.target.selectedOptions).map((opt) => opt.value))}
              className="w-full border rounded px-2 py-1"
            >
              {SECTORS.map(sector => (
                <option key={sector} value={sector}>{sector}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="mb-4">
          <label className="block text-sm font-medium mb-2">Keywords (comma-separated)</label>
          <input
            placeholder="e.g., climate tech, renewable energy, carbon removal"
            value={keywords}
            onChange={(e) => setKeywords(e.target.value)}
            className="w-full border rounded px-2 py-1"
          />
        </div>

        <Button 
          onClick={handleGenerateQueries}
          disabled={!keywords || loading}
        >
          {loading ? <Spinner /> : 'Generate Queries'}
        </Button>

        {error && (
          <div className="mt-4 p-3 bg-red-100 text-red-700 rounded">
            {error.message}
          </div>
        )}
      </Card>

      {/* Generated Queries */}
      {generatedQueries.length > 0 && (
        <Card>
          <h3 className="text-lg font-semibold mb-4">
            Generated Queries ({generatedQueries.length})
          </h3>
          <div>
            {generatedQueries.map(query => (
              <div 
                key={query.id}
                className="p-3 border rounded mb-2 hover:bg-gray-50"
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <h4 className="font-semibold">{query.name}</h4>
                    <p className="text-sm text-gray-600">{query.description}</p>
                  </div>
                  <Badge>{query.category}</Badge>
                </div>
                <code className="text-xs bg-gray-100 p-2 block rounded mb-2 overflow-auto">
                  {query.query}
                </code>
                <Button
                  size="1"
                  onClick={() => handleExecuteQuery(query.query)}
                  disabled={loading}
                >
                  {loading ? 'Executing...' : 'Execute'}
                </Button>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Search Results */}
      {searchExecuted && results.length > 0 && (
        <Card>
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold">
              Search Results ({results.length})
            </h3>
            <Button
              onClick={handleImportSelected}
              disabled={selectedResults.size === 0}
            >
              Import {selectedResults.size} Selected
            </Button>
          </div>

          <div>
            {results.map(result => (
              <div 
                key={result.url}
                className={`p-3 border rounded mb-2 cursor-pointer ${
                  selectedResults.has(result.url) ? 'bg-blue-50' : 'hover:bg-gray-50'
                }`}
                onClick={() => {
                  const newSelected = new Set(selectedResults);
                  if (newSelected.has(result.url)) {
                    newSelected.delete(result.url);
                  } else {
                    newSelected.add(result.url);
                  }
                  setSelectedResults(newSelected);
                }}
              >
                <div className="flex justify-between items-start mb-2">
                  <div className="flex-1">
                    <h4 className="font-semibold">{result.title}</h4>
                    <p className="text-sm text-gray-600">{result.snippet}</p>
                    <div className="mt-2 space-y-1">
                      {result.email && <p className="text-sm text-blue-600">{result.email}</p>}
                      {result.phone && <p className="text-sm text-green-600">{result.phone}</p>}
                    </div>
                  </div>
                  <div className="text-right">
                    <Progress value={result.relevanceScore} max={100} />
                    <Badge>{result.type}</Badge>
                  </div>
                </div>
                <a 
                  href={result.url} 
                  target="_blank" 
                  rel="noopener noreferrer"
                  className="text-xs text-gray-500 hover:underline"
                >
                  {result.domain}
                </a>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}