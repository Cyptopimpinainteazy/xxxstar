import { describe, it, expect } from 'vitest';
import { tokenizeX3, validateX3 } from '../src/x3/index';

describe('X3 Tokenizer', () => {
  it('tokenizes keywords', () => {
    const tokens = tokenizeX3('intent swap {');
    expect(tokens.some(t => t.type === 'keyword' && t.value === 'intent')).toBe(true);
  });

  it('tokenizes string literals', () => {
    const tokens = tokenizeX3('chain "ethereum"');
    expect(tokens.some(t => t.type === 'string' && t.value === '"ethereum"')).toBe(true);
  });

  it('tokenizes numbers', () => {
    const tokens = tokenizeX3('amount 1000');
    expect(tokens.some(t => t.type === 'number')).toBe(true);
  });

  it('handles comments', () => {
    const tokens = tokenizeX3('// this is a comment');
    expect(tokens.length).toBe(0);
  });
});

describe('X3 Validator', () => {
  it('detects unmatched opening brace', () => {
    const errors = validateX3('intent swap {\n  amount 1000\n');
    expect(errors.some(e => e.message.includes('Unmatched opening brace'))).toBe(true);
  });

  it('passes valid code', () => {
    const errors = validateX3('intent swap {\n  amount 1000\n}\n');
    const hasErrors = errors.length > 0;
    expect(hasErrors).toBe(false);
  });
});
