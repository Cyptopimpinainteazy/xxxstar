#!/usr/bin/env node
'use strict';

// Next 16.4.2 only uses globSync(..., { onlyDirectories: true }).
// tinyglobby avoids fast-glob's unpatched braces dependency, but additionally
// expands matched directories by default. Preserve Next's original root lookup.
// Bind the patch to the reviewed upstream bytes and refuse unknown versions.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const projectRequire = createRequire(path.resolve(process.argv[2] ?? '.', 'package.json'));
const entry = projectRequire.resolve('@next/eslint-plugin-next');
const target = path.join(path.dirname(entry), 'utils/get-root-dirs.js');
const source = fs.readFileSync(target, 'utf8');
const hash = (text) => crypto.createHash('sha256').update(text).digest('hex');
// Hashes of the registry's 16.4.2 helper and the single-option patch below.
const upstream = '886677432990a735e5ebdfb345ff1cbd40e264f9947a8432eeeb254b3a926bde';
const patched = 'a8304f8fee2763c6f2db1f1e1b8099792956ebb6088aabdcb47a843b9f331625';
if (hash(source) !== patched) {
  assert.equal(hash(source), upstream, 'Next root-glob helper changed: review before updating the patch');
  const result = source.replace('onlyDirectories: true', 'onlyDirectories: true,\n        expandDirectories: false');
  assert.equal(hash(result), patched);
  fs.writeFileSync(target, result);
}
console.log('Next ESLint root-glob compatibility patch applied');
