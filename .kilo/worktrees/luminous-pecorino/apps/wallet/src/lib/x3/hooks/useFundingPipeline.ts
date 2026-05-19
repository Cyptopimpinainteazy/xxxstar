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