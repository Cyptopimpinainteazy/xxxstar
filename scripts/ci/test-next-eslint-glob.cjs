#!/usr/bin/env node
'use strict';

// Exercise Next's actual root-directory discovery and navigation rule after the
// narrowly scoped fast-glob -> tinyglobby substitution. No rule is disabled.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createRequire } = require('node:module');
const { spawnSync } = require('node:child_process');

const project = path.resolve(process.argv[2]);
const projectRequire = createRequire(path.join(project, 'package.json'));
const entry = projectRequire.resolve('@next/eslint-plugin-next');
const pluginModule = projectRequire('@next/eslint-plugin-next');
const plugin = pluginModule.default ?? pluginModule;
const { getRootDirs } = projectRequire(path.join(path.dirname(entry), 'utils/get-root-dirs.js'));
const { Linter } = projectRequire('eslint');
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'x3-next-eslint-glob-'));

try {
  const site = path.join(tmp, 'apps', 'site');
  fs.mkdirSync(path.join(site, 'pages'), { recursive: true });
  fs.writeFileSync(path.join(site, 'pages', 'about.jsx'), 'export default function About() {}\n');
  fs.writeFileSync(path.join(tmp, 'apps', 'not-a-directory'), 'not an app');

  for (const rootDir of [path.join(tmp, 'apps', '*'), [path.join(tmp, 'apps', '{site,missing}')]]) {
    const settings = { next: { rootDir } };
    assert.deepEqual(getRootDirs({ cwd: tmp, settings }).map((dir) => path.resolve(dir)), [site]);
    const linter = new Linter();
    const config = [{
      files: ['**/*.jsx'],
      languageOptions: { parserOptions: { ecmaFeatures: { jsx: true } } },
      settings,
      plugins: { '@next/next': plugin },
      rules: { '@next/next/no-html-link-for-pages': 'error' },
    }];
    const internal = linter.verify('const nav = <a href="/about">About</a>;', config, 'navigation.jsx');
    assert.equal(internal.length, 1, JSON.stringify(internal));
    assert.equal(internal[0].ruleId, '@next/next/no-html-link-for-pages');
    assert.equal(internal[0].severity, 2);
    const external = linter.verify('const nav = <a href="https://example.com/about">About</a>;', config, 'navigation.jsx');
    assert.deepEqual(external, []);
  }
  const patchScript = path.join(__dirname, 'patch-next-eslint-glob.cjs');
  assert.equal(spawnSync(process.execPath, [patchScript, project]).status, 0, 'patch must be idempotent');
  const helper = path.join(path.dirname(entry), 'utils/get-root-dirs.js');
  const reviewed = fs.readFileSync(helper);
  const changed = Buffer.concat([reviewed, Buffer.from('\n// unreviewed upstream change\n')]);
  try {
    fs.writeFileSync(helper, changed);
    const refused = spawnSync(process.execPath, [patchScript, project]);
    assert.notEqual(refused.status, 0, 'an unreviewed helper must be refused');
    assert.deepEqual(fs.readFileSync(helper), changed, 'refusal must not write a partial patch');
  } finally {
    fs.writeFileSync(helper, reviewed);
  }
  console.log(`${project}: root globs, navigation rule and unreviewed-source refusal PASS`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
