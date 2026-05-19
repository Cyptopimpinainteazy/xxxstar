#!/usr/bin/env node

/**
 * Add unified navigation loader to all X3STAR HTML pages
 * This script adds <script src="js/x3-nav-loader.js"></script> to the head section
 * of all x3star-*.html files
 */

const fs = require('fs');
const path = require('path');

const FRONEND_DIR = '/home/lojak/Desktop/X3_ATOMIC_STAR/x3fronend';
const NAV_LOADER = '<script src="js/x3-nav-loader.js"></script>';

function addNavLoaderToFile(filePath) {
  try {
    let content = fs.readFileSync(filePath, 'utf-8');

    // Skip if already has nav loader
    if (content.includes('x3-nav-loader.js')) {
      return { status: 'skip', file: path.basename(filePath), reason: 'Already has nav loader' };
    }

    // Find the closing </head> tag and inject nav loader before it
    const headIndex = content.indexOf('</head>');
    if (headIndex === -1) {
      return { status: 'error', file: path.basename(filePath), reason: 'No closing </head> tag found' };
    }

    // Inject the nav loader script
    const updatedContent = content.slice(0, headIndex) + '  ' + NAV_LOADER + '\n' + content.slice(headIndex);

    // Write back
    fs.writeFileSync(filePath, updatedContent, 'utf-8');
    return { status: 'updated', file: path.basename(filePath) };
  } catch (error) {
    return { status: 'error', file: path.basename(filePath), reason: error.message };
  }
}

function main() {
  // Get all x3star-*.html files
  const files = fs.readdirSync(FRONEND_DIR)
    .filter(f => f.match(/^x3star-.*\.html$/) || f === 'test-rpc-connection.html')
    .map(f => path.join(FRONEND_DIR, f));

  console.log(`Processing ${files.length} HTML files...\n`);

  const results = files.map(file => addNavLoaderToFile(file));

  // Summary
  const updated = results.filter(r => r.status === 'updated');
  const skipped = results.filter(r => r.status === 'skip');
  const errors = results.filter(r => r.status === 'error');

  console.log(`✓ Updated: ${updated.length}`);
  console.log(`⊘ Skipped: ${skipped.length}`);
  console.log(`✗ Errors: ${errors.length}\n`);

  if (errors.length > 0) {
    console.log('Errors:');
    errors.forEach(e => console.log(`  ${e.file}: ${e.reason}`));
  }

  console.log('Done!');
}

main();
