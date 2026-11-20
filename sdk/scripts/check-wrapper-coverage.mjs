#!/usr/bin/env node
// Issue #461: fail CI when a contract function has no SDK wrapper
// Compares contract functions in the spec against SDK client methods and fails
// when a new function lands without either a wrapper or an explicit exclusion.
//
//   node scripts/check-wrapper-coverage.mjs --spec <path>  check coverage

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import ts from 'typescript';

const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CLIENT_CLASS = 'QuorumClient';

// Functions that exist in the contract but are intentionally not wrapped
const EXCLUSIONS = {
  governance: [
    'init', // internal initialization
  ],
  token: [
    'init', // internal initialization
  ],
};

function parse(filePath) {
  return ts.createSourceFile(filePath, readFileSync(filePath, 'utf8'), ts.ScriptTarget.Latest, true);
}

function getClientMethods() {
  const sourceFile = parse(path.join(sdkRoot, 'src/client.ts'));
  const methods = [];
  for (const statement of sourceFile.statements) {
    if (!ts.isClassDeclaration(statement) || statement.name?.text !== CLIENT_CLASS) continue;
    for (const member of statement.members) {
      if (!ts.isMethodDeclaration(member)) continue;
      const isPrivate = (member.modifiers ?? []).some(mod => mod.kind === ts.SyntaxKind.PrivateKeyword);
      const isProtected = (member.modifiers ?? []).some(mod => mod.kind === ts.SyntaxKind.ProtectedKeyword);
      if (!isPrivate && !isProtected) {
        methods.push(member.name.getText(sourceFile));
      }
    }
  }
  return methods;
}

function getContractFunctions(specPath) {
  try {
    const spec = JSON.parse(readFileSync(specPath, 'utf8'));
    return spec
      .filter(entry => entry.function_v0)
      .map(entry => entry.function_v0.name);
  } catch (error) {
    throw new Error(`Failed to read spec ${specPath}: ${error.message}`);
  }
}

function checkCoverage(specPath, contractName) {
  const clientMethods = new Set(getClientMethods());
  const contractFunctions = getContractFunctions(specPath);
  const excluded = new Set(EXCLUSIONS[contractName] || []);

  const missing = contractFunctions.filter(
    fn => !clientMethods.has(fn) && !excluded.has(fn)
  );

  if (missing.length > 0) {
    console.error(
      `\n❌ ${contractName} contract has ${missing.length} function(s) not wrapped in SDK:`
    );
    missing.forEach(fn => console.error(`   - ${fn}`));
    console.error(
      '\nAdd a wrapper method to QuorumClient or add an entry to EXCLUSIONS with a reason.\n'
    );
    return false;
  }

  console.log(`✓ ${contractName}: all ${contractFunctions.length} contract functions are covered`);
  return true;
}

const specArg = process.argv.indexOf('--spec');
if (specArg === -1 || !process.argv[specArg + 1]) {
  console.error('Usage: node check-wrapper-coverage.mjs --spec <path>');
  process.exit(1);
}

const specPath = process.argv[specArg + 1];
const success = checkCoverage(specPath, path.basename(specPath).match(/quorum-(\w+)/)?.[1] || 'unknown');
process.exit(success ? 0 : 1);
