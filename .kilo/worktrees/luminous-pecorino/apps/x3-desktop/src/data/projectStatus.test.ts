import { describe, expect, it } from 'vitest';
import {
  DESKTOP_READINESS_STATUS,
  FEATURE_STATUSES,
  summarizeFeatureModes,
} from './projectStatus';

describe('project status snapshot', () => {
  it('keeps desktop readiness guarded until proof gates are wired', () => {
    expect(DESKTOP_READINESS_STATUS.status).toBe('guarded');
    expect(DESKTOP_READINESS_STATUS.gaps).toContain(
      'Backend app registry currently returns an empty list, so the frontend falls back to a static registry.',
    );
  });

  it('summarizes feature modes shown in the desktop readiness panel', () => {
    const counts = summarizeFeatureModes(FEATURE_STATUSES);

    expect(counts.LIVE_TESTNET).toBe(2);
    expect(counts.GUARDED_TESTNET).toBe(3);
    expect(counts.SIM_TESTNET).toBe(1);
    expect(counts.DISABLED_BLOCKED).toBe(0);
  });
});