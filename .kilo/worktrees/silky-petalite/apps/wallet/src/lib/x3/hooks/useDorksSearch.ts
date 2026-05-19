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

export interface InvestorMatch {
  investorId: string;
  matchScore: number;
  sectorAlignment: number;
  stageAlignment: number;
  ticketSizeAlignment: number;
  locationAlignment: number;
  contactProbability: number;
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