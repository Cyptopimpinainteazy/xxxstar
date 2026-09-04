import assert from 'assert';
import { cosineSim, normalizeThink } from '../../../../src/llm/ollama.js';

assert.strictEqual(cosineSim([1, 0], [0, 1]), 0, 'orthogonal vectors should score 0');
assert.strictEqual(cosineSim([1, 1], [1, 1]), 1, 'identical vectors should score 1');
assert.strictEqual(Math.round(cosineSim([1, 2], [2, 4]) * 1000) / 1000, 1, 'parallel vectors should score 1');

assert.strictEqual(normalizeThink(false, undefined), undefined, 'no think and no level should return undefined');
assert.strictEqual(normalizeThink(true, undefined), true, 'true think should return true');
assert.strictEqual(normalizeThink(false, 2), 2, 'explicit thinkLevel should override false think');
assert.strictEqual(normalizeThink(true, 0), 0, 'explicit zero thinkLevel should be preserved');

console.log('OLLAMA helper smoke tests passed.');
