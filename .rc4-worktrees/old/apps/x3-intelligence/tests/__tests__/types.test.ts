import { describe, it, expect, vi } from 'vitest';
import { IntentState, AgentStatus, SlashSeverity } from '../../src/types';

describe('Types', () => {
  describe('IntentState', () => {
    it('should have all required states', () => {
      expect(IntentState.Submitted).toBe('Submitted');
      expect(IntentState.RouteBound).toBe('RouteBound');
      expect(IntentState.Executing).toBe('Executing');
      expect(IntentState.Executed).toBe('Executed');
      expect(IntentState.Finalized).toBe('Finalized');
      expect(IntentState.Slashed).toBe('Slashed');
      expect(IntentState.Cancelled).toBe('Cancelled');
      expect(IntentState.Expired).toBe('Expired');
    });
  });

  describe('AgentStatus', () => {
    it('should have all required statuses', () => {
      expect(AgentStatus.Active).toBe('Active');
      expect(AgentStatus.Suspended).toBe('Suspended');
      expect(AgentStatus.Deregistered).toBe('Deregistered');
      expect(AgentStatus.Deactivated).toBe('Deactivated');
    });
  });

  describe('SlashSeverity', () => {
    it('should have all required severities', () => {
      expect(SlashSeverity.Minor).toBe('Minor');
      expect(SlashSeverity.Moderate).toBe('Moderate');
      expect(SlashSeverity.Major).toBe('Major');
      expect(SlashSeverity.Critical).toBe('Critical');
    });
  });
});
