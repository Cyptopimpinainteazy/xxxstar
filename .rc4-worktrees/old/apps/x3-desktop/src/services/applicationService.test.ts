import { describe, expect, it } from 'vitest';
import { DEFAULT_APPLICATIONS } from './applicationService';

describe('DEFAULT_APPLICATIONS', () => {
  it('does not expose stale unsupported launch claims in desktop copy', () => {
    const descriptions = DEFAULT_APPLICATIONS.map((app) => app.description ?? '').join('\n');

    expect(descriptions).not.toContain('103 EVM chains in 6 seconds');
    expect(descriptions).not.toContain('$89M');
    expect(descriptions).not.toContain('5-15% APY');
  });
});