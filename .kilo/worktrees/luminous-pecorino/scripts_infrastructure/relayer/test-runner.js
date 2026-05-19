#!/usr/bin/env node
// Lightweight test runner to avoid mocha/loader race issues by requiring tests sequentially

require('ts-node/register');
const Mocha = require('mocha');
const glob = require('glob');

const mocha = new Mocha({ reporter: 'spec', timeout: process.env.TEST_TIMEOUT ? parseInt(process.env.TEST_TIMEOUT) : 20000 });

const files = glob.sync('tests/**/*.test.ts');
if (files.length === 0) {
    console.error('No test files found');
    process.exit(1);
}

for (const file of files) {
    mocha.addFile(file);
}

mocha.run((failures) => {
    if (failures) {
        console.error(`${failures} tests failed`);
        process.exit(1);
    } else {
        console.log('All tests passed');
        process.exit(0);
    }
});
