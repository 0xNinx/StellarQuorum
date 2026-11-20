#!/usr/bin/env node
/**
 * #405 — verify that every initialize() parameter in contract-specs.yml has a
 * matching --flag in scripts/deploy.sh. CI fails if they drift.
 */
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "node:util"; // native YAML not in Node; use inline parser below

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const specPath = resolve(root, ".github/workflows/contract-specs.yml");
const deployPath = resolve(root, "scripts/deploy.sh");

const specSrc = readFileSync(specPath, "utf8");
const deploySrc = readFileSync(deployPath, "utf8");

// Extract initialize parameter names from lines like `  - name: admin`
// that appear after an `initialize:` block in the YAML spec.
const initBlocks = [...specSrc.matchAll(/initialize:\s*\n((?:\s+.*\n)*)/g)];
const specParams = new Set();
for (const [, block] of initBlocks) {
  for (const m of block.matchAll(/name:\s*(\w+)/g)) {
    specParams.add(m[1]);
  }
}

// Extract --flag names from the deploy.sh initialize calls.
const deployFlags = new Set(
  [...deploySrc.matchAll(/--(\w+)\s+"\$\{?[A-Z_]+\}?"/g)].map((m) => m[1])
);

let failed = false;
for (const param of specParams) {
  if (!deployFlags.has(param)) {
    console.error(`[check-deploy-flags] MISSING: spec param '${param}' has no --${param} flag in deploy.sh`);
    failed = true;
  }
}

if (failed) {
  console.error("\nUpdate scripts/deploy.sh to match the initialize() signature in contract-specs.yml.");
  process.exit(1);
}
console.log(`[check-deploy-flags] OK — ${specParams.size} param(s) all present in deploy.sh`);
