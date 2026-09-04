/**
 * Smoke test: verifies the public package API imports and basic invariants so
 * the documented `npm test` command runs at least one real, meaningful check.
 */
import {
  ApiClientError,
  AuthenticationError,
  NotFoundError,
  ValidationError,
} from '../client';
import { DAppType } from '../types';
import type { FoundryClientConfig } from '../types';

describe('foundry-sdk public API', () => {
  it('exposes typed error classes with HTTP metadata', () => {
    const err = new AuthenticationError('bad key', 'req-1');
    expect(err).toBeInstanceOf(ApiClientError);
    expect(err.name).toBe('AuthenticationError');
    expect(err.message).toBe('bad key');
    expect(err.statusCode).toBe(401);
    expect(err.code).toBe('UNAUTHORIZED');
    expect(err.requestId).toBe('req-1');

    const nf = new NotFoundError('missing');
    expect(nf.statusCode).toBe(404);
    expect(nf).toBeInstanceOf(ApiClientError);

    const ve = new ValidationError('bad input');
    expect(ve.statusCode).toBe(422);
    expect(ve.code).toBe('VALIDATION_ERROR');
    expect(ve).toBeInstanceOf(ApiClientError);
  });

  it('exposes DAppType enum values used across the SDK', () => {
    expect(DAppType.EVM).toBe('evm');
    expect(DAppType.SVM).toBe('svm');
    expect(DAppType.Comit).toBe('comit');
  });

  it('compiles against the FoundryClientConfig type contract', () => {
    const cfg: FoundryClientConfig = {
      apiUrl: 'https://api.foundry.x3.ai',
      chainId: 1,
    };
    expect(cfg.apiUrl).toBe('https://api.foundry.x3.ai');
    expect(cfg.chainId).toBe(1);
  });
});
