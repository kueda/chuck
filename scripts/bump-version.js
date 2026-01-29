#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';

const args = process.argv.slice(2);
const DRY_RUN = args.includes('--dry-run');
const BUMP_TYPE = args.find((a) => ['patch', 'minor', 'major'].includes(a));

if (!BUMP_TYPE) {
  console.error(
    'Usage: node scripts/bump-version.js <patch|minor|major> [--dry-run]',
  );
  process.exit(1);
}

function bumpVersion(version, type) {
  const [major, minor, patch] = version.split('.').map(Number);
  switch (type) {
    case 'major':
      return `${major + 1}.0.0`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'patch':
      return `${major}.${minor}.${patch + 1}`;
  }
}

function updateJson(filePath, newVersion) {
  const content = JSON.parse(readFileSync(filePath, 'utf8'));
  content.version = newVersion;
  writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
}

function updateCargoToml(filePath, newVersion) {
  let content = readFileSync(filePath, 'utf8');
  content = content.replace(
    /^version = "[\d.]+"$/m,
    `version = "${newVersion}"`,
  );
  writeFileSync(filePath, content);
}

const FILES = [
  'package.json',
  'src-tauri/tauri.conf.json',
  'src-tauri/Cargo.toml',
  'chuck-core/Cargo.toml',
  'chuck-cli/Cargo.toml',
];

// Read current version
const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
const currentVersion = pkg.version;
const newVersion = bumpVersion(currentVersion, BUMP_TYPE);
const tag = `v${newVersion}`;

console.log(`Bumping version: ${currentVersion} → ${newVersion}`);

if (DRY_RUN) {
  console.log('\n[dry-run] Would update:');
  FILES.forEach((f) => console.log(`  ${f}`));
  console.log('  package-lock.json (via npm install --package-lock-only)');
  console.log(`\n[dry-run] Would commit with message: ${tag}`);
  console.log(`[dry-run] Would create tag: ${tag}`);
  process.exit(0);
}

// Update all files
updateJson('package.json', newVersion);
updateJson('src-tauri/tauri.conf.json', newVersion);
updateCargoToml('src-tauri/Cargo.toml', newVersion);
updateCargoToml('chuck-core/Cargo.toml', newVersion);
updateCargoToml('chuck-cli/Cargo.toml', newVersion);

// Sync package-lock.json
execSync('npm install --package-lock-only', { stdio: 'inherit' });

// Git commit and tag (--no-verify skips hooks)
execSync(`git add ${FILES.join(' ')} package-lock.json`);
execSync(`git commit --no-verify -m "${tag}"`);
execSync(`git tag -a ${tag} -m "${tag}"`);

console.log(`Committed and tagged ${tag}`);
