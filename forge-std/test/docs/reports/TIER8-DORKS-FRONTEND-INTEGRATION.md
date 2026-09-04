# TIER 8: Google Dorks Search & Fundraising Dashboard
## Frontend Integration Guide

Complete guide for wiring the Google Dorks search system and funding/investor discovery features into your React frontend.

---

## Table of Contents

1. [Overview & Architecture](#overview--architecture)
2. [New React Hooks](#new-react-hooks)
3. [Component Examples](#component-examples)
4. [Integration Patterns](#integration-patterns)
5. [Search Features](#search-features)
6. [Funding Dashboard](#funding-dashboard)
7. [Testing & Validation](#testing--validation)

---

## Overview & Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    React Frontend                           │
│                                                               │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────┐  │
│  │ DorksSearch      │  │ FundingDashboard │  │ Investor │  │
│  │ Component        │  │ Component        │  │ Matches  │  │
│  └────────┬─────────┘  └────────┬─────────┘  └────┬─────┘  │
│           │                    │                   │         │
│           └────────────────────┼───────────────────┘         │
│                                │                             │
│                        ┌───────▼────────┐                   │
│                        │ useDorksSearch  │                   │
│                        │ Custom Hook     │                   │
│                        └───────┬────────┘                    │
├─────────────────────────────────┼──────────────────────────┤
│                    Tauri Bridge  │                          │
├─────────────────────────────────┼──────────────────────────┤
│              Backend (Rust)      │                          │
│                                  ▼                          │
│    ┌──────────────────────────────────────┐               │
│    │   dorks_commands.rs (15+ Handlers)   │               │
│    │                                       │               │
│    │  • crm_search_investors_by_sector    │               │
│    │  • crm_search_grant_opportunities    │               │
│    │  • crm_generate_dorks_query          │               │
│    │  • crm_execute_dorks_search          │               │
│    │  • crm_auto_generate_search_queries  │               │
│    │  • crm_import_dorks_result_as_contact│               │
│    │  • crm_bulk_import_dorks_results     │               │
│    │  • crm_get_investor_matches          │               │
│    │  • ... 7+ more analytics commands    │               │
│    └──────────────────────────────────────┘               │
│                   │                                        │
│                   ▼                                        │
│    ┌──────────────────────────────────────┐               │
│    │      SQLite Database                 │               │
│    │                                       │               │
│    │  crm_investors          / 8 tables  │               │
│    │  crm_grants             / 100+ cols│               │
│    │  crm_dorks_queries      / 50+ idx  │               │
│    │  crm_dorks_results                  │               │
│    │  crm_dorks_campaigns                │               │
│    │  crm_funding_rounds                 │               │
│    │  crm_funding_analytics              │               │
│    │  crm_investor_meetings              │               │
│    └──────────────────────────────────────┘               │
└────────────────────────────────────────────────────────────┘
```

---

## New React Hooks

### `useDorksSearch` Hook

Custom hook for executing Google Dorks searches from React components.

```typescript
// src/hooks/useDorksSearch.ts

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

export interface GoogleDorksQuery {
  id: string;
  name: string;
  category: string;
  query: string;
  description?: string;
  tags?: string[];
}

export interface GoogleDorksResult {
  title: string;
  url: string;
  snippet: string;
  domain: string;
  email?: string;
  phone?: string;
  type: 'investor' | 'grant' | 'accelerator' | 'founder' | 'competitor';
  relevanceScore: number;
  saved: boolean;
}

interface UseDorksSearchResult {
  // State
  queries: GoogleDorksQuery[];
  results: GoogleDorksResult[];
  loading: boolean;
  error: Error | null;
  generatedQueries: GoogleDorksQuery[];
  
  // Actions
  searchInvestorsBySector: (sector: string, location?: string) => Promise<GoogleDorksQuery[]>;
  searchGrantOpportunities: (sector: string, amountMin?: number) => Promise<GoogleDorksQuery[]>;
  autogenerateSearachQueries: (objective: string, keywords: string[]) => Promise<GoogleDorksQuery[]>;
  executeDorksSearch: (query: string, limit?: number) => Promise<GoogleDorksResult[]>;
  importResultAsContact: (result: GoogleDorksResult) => Promise<string>; // returns contact_id
  bulkImportResults: (results: GoogleDorksResult[]) => Promise<number>; // returns count
  getInvestorMatches: (sectors: string[], amount: number) => Promise<InvestorMatch[]>;
  getSearchHistory: () => Promise<GoogleDorksQuery[]>;
  
  // Utilities
  searchResultsByType: (type: string) => GoogleDorksResult[];
}

export function useDorksSearch(): UseDorksSearchResult {
  const [queries, setQueries] = useState<GoogleDorksQuery[]>([]);
  const [results, setResults] = useState<GoogleDorksResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [generatedQueries, setGeneratedQueries] = useState<GoogleDorksQuery[]>([]);

  const searchInvestorsBySector = useCallback(
    async (sector: string, location?: string) => {
      setLoading(true);
      setError(null);
      try {
        const queries = await invoke<GoogleDorksQuery[]>(
          'crm_search_investors_by_sector',
          { sector, location }
        );
        setQueries(queries);
        return queries;
      } catch (err) {
        setError(err as Error);
        return [];
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const searchGrantOpportunities = useCallback(
    async (sector: string, amountMin?: number) => {
      setLoading(true);
      setError(null);
      try {
        const queries = await invoke<GoogleDorksQuery[]>(
          'crm_search_grant_opportunities',
          { sector, amount_min: amountMin }
        );
        setQueries(queries);
        return queries;
      } catch (err) {
        setError(err as Error);
        return [];
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const autogenerateSearachQueries = useCallback(
    async (objective: string, keywords: string[]) => {
      setLoading(true);
      setError(null);
      try {
        const generated = await invoke<GoogleDorksQuery[]>(
          'crm_auto_generate_search_queries',
          { objective, keywords }
        );
        setGeneratedQueries(generated);
        return generated;
      } catch (err) {
        setError(err as Error);
        return [];
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const executeDorksSearch = useCallback(
    async (query: string, limit = 50) => {
      setLoading(true);
      setError(null);
      try {
        const searchResults = await invoke<GoogleDorksResult[]>(
          'crm_execute_dorks_search',
          { query, limit_results: limit }
        );
        setResults(searchResults);
        return searchResults;
      } catch (err) {
        setError(err as Error);
        return [];
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const importResultAsContact = useCallback(
    async (result: GoogleDorksResult) => {
      try {
        const contactId = await invoke<string>(
          'crm_import_dorks_result_as_contact',
          { result }
        );
        // Update results to mark as saved
        setResults(prev =>
          prev.map(r =>
            r.url === result.url ? { ...r, saved: true } : r
          )
        );
        return contactId;
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  const bulkImportResults = useCallback(
    async (resultsToImport: GoogleDorksResult[]) => {
      try {
        const count = await invoke<number>(
          'crm_bulk_import_dorks_results',
          { results: resultsToImport }
        );
        // Mark all as saved
        setResults(prev =>
          prev.map(r =>
            resultsToImport.find(ri => ri.url === r.url)
              ? { ...r, saved: true }
              : r
          )
        );
        return count;
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  const getInvestorMatches = useCallback(
    async (sectors: string[], amount: number) => {
      setLoading(true);
      setError(null);
      try {
        const matches = await invoke<InvestorMatch[]>(
          'crm_get_investor_matches',
          { sectors, seeking_amount: amount }
        );
        return matches;
      } catch (err) {
        setError(err as Error);
        return [];
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const getSearchHistory = useCallback(
    async () => {
      try {
        const history = await invoke<GoogleDorksQuery[]>(
          'crm_get_dorks_search_history',
          {}
        );
        return history;
      } catch (err) {
        setError(err as Error);
        return [];
      }
    },
    []
  );

  const searchResultsByType = useCallback(
    (type: string) => {
      return results.filter(r => r.type === type);
    },
    [results]
  );

  return {
    queries,
    results,
    loading,
    error,
    generatedQueries,
    searchInvestorsBySector,
    searchGrantOpportunities,
    autogenerateSearachQueries,
    executeDorksSearch,
    importResultAsContact,
    bulkImportResults,
    getInvestorMatches,
    getSearchHistory,
    searchResultsByType,
  };
}
```

### `useFundingPipeline` Hook

Hook for managing funding rounds and investor tracking.

```typescript
// src/hooks/useFundingPipeline.ts

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

export interface FundingRound {
  id: string;
  roundName: string;
  roundType: string;
  status: 'planning' | 'active' | 'closed';
  targetAmountUsd: number;
  raisedAmountUsd: number;
  leadInvestor?: string;
  investorsCount: number;
  successProbability: number;
  estimatedCloseDate?: Date;
}

export interface InvestorMatch {
  investorId: string;
  matchScore: number;
  sectorAlignment: number;
  stageAlignment: number;
  ticketSizeAlignment: number;
  locationAlignment: number;
  contactProbability: number;
}

interface UseFundingPipelineResult {
  fundingRounds: FundingRound[];
  currentRound: FundingRound | null;
  analytics: any;
  loading: boolean;
  error: Error | null;

  getFundingAnalytics: () => Promise<any>;
  createFundingRound: (round: Partial<FundingRound>) => Promise<string>;
  updateFundingRound: (roundId: string, updates: Partial<FundingRound>) => Promise<void>;
  addInvestorToRound: (roundId: string, investorId: string) => Promise<void>;
  recordInvestorMeeting: (investorId: string, roundId: string, meeting: any) => Promise<string>;
}

export function useFundingPipeline(): UseFundingPipelineResult {
  const [fundingRounds, setFundingRounds] = useState<FundingRound[]>([]);
  const [currentRound, setCurrentRound] = useState<FundingRound | null>(null);
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const getFundingAnalytics = useCallback(
    async () => {
      setLoading(true);
      try {
        const data = await invoke('crm_get_funding_analytics', {});
        setAnalytics(data);
        return data;
      } catch (err) {
        setError(err as Error);
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const createFundingRound = useCallback(
    async (round: Partial<FundingRound>) => {
      try {
        const roundId = await invoke<string>(
          'crm_create_funding_round',
          { round }
        );
        return roundId;
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  const updateFundingRound = useCallback(
    async (roundId: string, updates: Partial<FundingRound>) => {
      try {
        await invoke('crm_update_funding_round', { round_id: roundId, updates });
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  const addInvestorToRound = useCallback(
    async (roundId: string, investorId: string) => {
      try {
        await invoke('crm_add_investor_to_round', {
          round_id: roundId,
          investor_id: investorId,
        });
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  const recordInvestorMeeting = useCallback(
    async (investorId: string, roundId: string, meeting: any) => {
      try {
        const meetingId = await invoke<string>(
          'crm_record_investor_meeting',
          { investor_id: investorId, funding_round_id: roundId, meeting }
        );
        return meetingId;
      } catch (err) {
        setError(err as Error);
        throw err;
      }
    },
    []
  );

  return {
    fundingRounds,
    currentRound,
    analytics,
    loading,
    error,
    getFundingAnalytics,
    createFundingRound,
    updateFundingRound,
    addInvestorToRound,
    recordInvestorMeeting,
  };
}
```

---

## Component Examples

### DorksSearchPanel Component

Complete search interface for finding investors and grants.

```typescript
// src/components/crm/DorksSearchPanel.tsx

import React, { useState } from 'react';
import { useDorksSearch } from '../../hooks/useDorksSearch';
import { Button, Card, Select, Input, List, Progress, Badge, Spinner } from '@radix-ui/themes';

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
            <Select value={objective} onValueChange={setObjective}>
              {OBJECTIVES.map(opt => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </Select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Focus Sectors</label>
            <Select 
              multiple 
              value={sectors}
              onValueChange={setSectors}
            >
              {SECTORS.map(sector => (
                <option key={sector} value={sector}>{sector}</option>
              ))}
            </Select>
          </div>
        </div>

        <div className="mb-4">
          <label className="block text-sm font-medium mb-2">Keywords (comma-separated)</label>
          <Input
            placeholder="e.g., climate tech, renewable energy, carbon removal"
            value={keywords}
            onChange={(e) => setKeywords(e.target.value)}
            className="w-full"
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
          <List>
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
                  size="small"
                  onClick={() => handleExecuteQuery(query.query)}
                  disabled={loading}
                >
                  {loading ? 'Executing...' : 'Execute'}
                </Button>
              </div>
            ))}
          </List>
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

          <List>
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
          </List>
        </Card>
      )}
    </div>
  );
}
```

### FundingDashboard Component

Complete funding pipeline overview and analytics.

```typescript
// src/components/crm/FundingDashboard.tsx

import React, { useEffect, useState } from 'react';
import { useFundingPipeline, useDorksSearch } from '../../hooks';
import { Card, Progress, Infobox, Button, Badge, Chart, Grid } from '@radix-ui/themes';
import { PieChart, LineChart, BarChart, Legend, Tooltip, ResponsiveContainer } from 'recharts';

export function FundingDashboard() {
  const { analytics, getFundingAnalytics } = useFundingPipeline();
  const { getInvestorMatches } = useDorksSearch();
  const [investorMatches, setInvestorMatches] = useState<any[]>([]);

  useEffect(() => {
    getFundingAnalytics();
    getInvestorMatches(['AI', 'SaaS'], 1000000).then(setInvestorMatches);
  }, []);

  if (!analytics) return <div>Loading...</div>;

  const fundingPercent = (analytics.totalRaisedUsd / analytics.totalTargetUsd) * 100;
  const sourcesData = [
    { name: 'VC', value: analytics.fromVcUsd },
    { name: 'Angel', value: analytics.fromAngelUsd },
    { name: 'Grants', value: analytics.fromGrantsUsd },
    { name: 'Corporate', value: analytics.fromCorporateUsd },
  ];

  const pipelineData = [
    { name: 'In Pipeline', value: analytics.investorsInPipeline },
    { name: 'Interested', value: analytics.investorsInterested },
    { name: 'Committed', value: analytics.investorsCommitted },
  ];

  return (
    <div className="space-y-6">
      {/* Key Metrics */}
      <Grid columns="4" gap="4">
        <Card>
          <h4 className="text-sm text-gray-600">Total Target</h4>
          <p className="text-2xl font-bold">${(analytics.totalTargetUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Total Raised</h4>
          <p className="text-2xl font-bold">${(analytics.totalRaisedUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Funding Gap</h4>
          <p className="text-2xl font-bold">${(analytics.fundingGapUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Success Probability</h4>
          <p className="text-2xl font-bold">{analytics.successProbabilityPercentage.toFixed(0)}%</p>
        </Card>
      </Grid>

      {/* Funding Progress */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Funding Progress</h3>
        <Progress 
          value={fundingPercent}
          max={100}
          className="mb-4"
        />
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-sm text-gray-600">Progress</p>
            <p className="text-xl font-bold">{fundingPercent.toFixed(1)}%</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Months to Close</p>
            <p className="text-xl font-bold">{analytics.monthsToClose}</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Estimated Close</p>
            <p className="text-xl font-bold">{new Date(analytics.estimatedCloseDate).toLocaleDateString()}</p>
          </div>
        </div>
      </Card>

      {/* Sources Breakdown */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Funding Sources</h3>
        <ResponsiveContainer width="100%" height={300}>
          <PieChart>
            <Pie 
              data={sourcesData}
              dataKey="value"
              nameKey="name"
              label
            />
            <Tooltip formatter={(value) => `$${(value / 1000000).toFixed(1)}M`} />
            <Legend />
          </PieChart>
        </ResponsiveContainer>
      </Card>

      {/* Top Investor Matches */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Top Investor Matches</h3>
        {investorMatches.slice(0, 5).map(match => (
          <div key={match.investorId} className="p-3 border rounded mb-2">
            <div className="flex justify-between items-center mb-2">
              <h4 className="font-semibold">{match.investorName}</h4>
              <Badge color={match.matchScore > 80 ? 'green' : 'yellow'}>
                {match.matchScore}/100
              </Badge>
            </div>
            <Progress value={match.matchScore} max={100} />
            <div className="grid grid-cols-4 gap-2 mt-2 text-xs">
              <div>
                <p className="text-gray-600">Sector</p>
                <p className="font-semibold">{(match.sectorAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Stage</p>
                <p className="font-semibold">{(match.stageAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Ticket</p>
                <p className="font-semibold">{(match.ticketSizeAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Contact Prob</p>
                <p className="font-semibold">{(match.contactProbability * 100).toFixed(0)}%</p>
              </div>
            </div>
            <Button size="small" className="w-full mt-2" variant="outline">
              View Profile
            </Button>
          </div>
        ))}
      </Card>

      {/* Investor Pipeline Summary */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Investor Pipeline</h3>
        <div className="grid grid-cols-3 gap-4">
          {pipelineData.map(item => (
            <div key={item.name} className="text-center">
              <p className="text-sm text-gray-600 mb-2">{item.name}</p>
              <p className="text-3xl font-bold">{item.value}</p>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
```

---

## Integration Patterns

### Pattern 1: Search → Import → Track

```typescript
// Typical workflow: find investors, import as contacts, track meetings

async function fundraiseWorkflow() {
  // 1. Find investors
  const investors = await useDorksSearch().searchInvestorsBySector('AI', 'San Francisco');
  
  // 2. Execute search
  const results = await useDorksSearch().executeDorksSearch(investors[0].query);
  
  // 3. Filter high-quality results
  const topResults = results.filter(r => r.relevanceScore > 80);
  
  // 4. Bulk import as contacts
  const importedCount = await useDorksSearch().bulkImportResults(topResults);
  
  // 5. Get investor matches and contact probability
  const matches = await useDorksSearch().getInvestorMatches(['AI'], 1000000);
  
  // 6. Schedule meetings with high-probability matches
  for (const match of matches.filter(m => m.matchScore > 85)) {
    await useFundingPipeline().recordInvestorMeeting(
      match.investorId,
      currentRound.id,
      { type: 'intro', date: new Date() }
    );
  }
}
```

### Pattern 2: Grant Discovery & Application Tracking

```typescript
async function grantDiscoveryWorkflow() {
  // 1. Search for relevant grants
  const grants = await useDorksSearch().searchGrantOpportunities('Climate Tech', 100000);
  
  // 2. Execute each search
  const allResults: GoogleDorksResult[] = [];
  for (const grant of grants) {
    const results = await useDorksSearch().executeDorksSearch(grant.query);
    allResults.push(...results);
  }
  
  // 3. Import grant opportunities
  const importedCount = await useDorksSearch().bulkImportResults(
    allResults.filter(r => r.type === 'grant')
  );
  
  console.log(`Found and imported ${importedCount} grant opportunities`);
}
```

### Pattern 3: Campaign-Based Search

```typescript
async function launchSearchCampaign(objective: string, keywords: string[]) {
  const { useDorksSearch } = useHooks();
  
  // 1. Auto-generate queries
  const queries = await useDorksSearch().autogenerateSearachQueries(objective, keywords);
  
  // 2. Execute each query
  const allResults: GoogleDorksResult[] = [];
  for (const query of queries) {
    const results = await useDorksSearch().executeDorksSearch(query.query);
    allResults.push(...results);
  }
  
  // 3. Organize by type and relevance
  const byType = {
    investors: allResults.filter(r => r.type === 'investor').sort((a, b) => b.relevanceScore - a.relevanceScore),
    grants: allResults.filter(r => r.type === 'grant'),
    accelerators: allResults.filter(r => r.type === 'accelerator'),
  };
  
  return byType;
}
```

---

## Search Features

### Investor Search
Finds venture capitalists, angel investors, accelerators, and corporate venture teams.

```typescript
// Search Parameters
{
  sector: "AI/ML" | "SaaS" | "ClimaTech" | ...
  location: "San Francisco" | "New York" | "Global" | ...
  investmentStage: "seed" | "series_a" | "series_b" | ...
  minimumTicketSize: 100000 // USD
}

// Generated Queries include:
// - Crunchbase investor profiles
// - AngelList angel investor listings
// - LinkedIn venture capitalist search
// - Twitter investor discovery
// - Industry-specific VC directories
```

### Grant Discovery
Finds federal grants, foundation grants, corporate grants, and award programs.

```typescript
// Search Parameters
{
  sector: "Climate Tech" | "AI" | "Healthcare" | ...
  minimumAmount: 100000 // USD
  country: "USA" | "Global" | ...
}

// Generated Queries include:
// - Grants.gov search
// - SBIR Phase I & II (10+ queries)
// - NSF funding opportunities
// - Foundation grants (Philanthropy.org, GrantStation)
// - Corporate grants (Google.org, Stripe Climate, etc.)
// - Government research funding
```

### Competitor Intelligence
Analyzes market, funding rounds, and exits.

```typescript
// Search Parameters
{
  sector: "Web3" | "Climate" | ...
  market: "DeFi" | "NFTs" | ...
}

// Generated Queries include:
// - Recent funding announcements
// - Tech stack research
// - Market cap analysis
// - Recent exits and IPOs
```

---

## Funding Dashboard

Track your complete fundraising pipeline:

- **Funding Progress**: Visual breakdown of raised vs. target
- **Investor Pipeline**: In pipeline → Interested → Committed
- **Sources Breakdown**: VC vs. Angel vs. Grants vs. Corporate
- **Top Matches**: AI-ranked investor matches with alignment scores
- **Forecast**: Estimated close date and success probability
- **Meeting History**: All investor interactions with outcomes

---

## Testing & Validation

### Test Suite

All tests in `smoke-tests.spec.ts` and `unit_integration_tests.rs`:

```bash
# Run E2E tests
npm run test:e2e -- --grep "Dorks|Investor|Grant|Funding"

# Run unit tests
cargo test crm::dorks --lib
cargo test crm::funding --lib
```

### Manual Validation Checklist

- [ ] Generate 5+ queries for "AI" sector
- [ ] Execute first query and verify 10+ results
- [ ] Import 5 results as contacts
- [ ] Verify contacts appear in CRM
- [ ] Get investor matches for $1M seed round
- [ ] Verify match scores 0-100
- [ ] Record investor meeting
- [ ] Verify meeting appears in funding round history
- [ ] Search grants for "Climate Tech"
- [ ] Import grant results
- [ ] View funding dashboard
- [ ] Verify all metrics calculate correctly

---

## Configuration

```typescript
// .env
GOOGLE_CSE_ID=your-custom-search-engine-id  // Optional: for production
GOOGLE_CSE_KEY=your-api-key                  // Optional: for production

// src/config/dorks.ts
export const DORKS_CONFIG = {
  resultsPerQuery: 50,
  autoImportThreshold: 80, // relevance score
  refreshIntervalDays: 7,   // re-search grants
  batchImportSize: 25,
};
```

---

## Troubleshooting

**Q: Searches returning no results**
A: Demo mode returns fixture data. For production, set up Google Custom Search API (see Configuration).

**Q: Gmail extraction not working**
A: Gmail hides email addresses behind login. Use LinkedIn/Crunchbase searches instead.

**Q: Investor match scores all similar**
A: Run `crm_get_dorks_analytics()` to see score distribution. May need to adjust match algorithm thresholds.

**Q: Performance slow on bulk import**
A: Batch size of 25-50 recommended. Use `crm_bulk_import_dorks_results()` not individual imports.

---

## Success Metrics

✅ Find 100+ relevant investor profiles per search campaign
✅ Import 50+ new contacts per week from searches
✅ Match company profile to top 20 investors by relevance
✅ Discover 30+ grant opportunities per sector
✅ Track investor meetings and outcomes
✅ Forecast funding close date with 70%+ accuracy

