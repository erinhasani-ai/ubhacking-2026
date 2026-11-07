// Checks .env.example holds only names (no values) and that every env read in the Rust source is listed there.
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const LINE = /^([A-Z][A-Z0-9_]*)=$/;
const ENV_READ = /(?:env::var|env::var_os|dotenvy::var)\(\s*"([A-Z][A-Z0-9_]*)"/g;
// shortcut: only scans Rust env::var/dotenvy::var reads, extend when the web UI reads env vars

// Returns the names declared in an env example, and the line numbers of lines that are not "NAME=".
function parseExample(text) {
  const names = new Set();
  const bad = [];
  text.split('\n').forEach((raw, i) => {
    const line = raw.replace(/\r$/, '');
    if (line.trim() === '' || line.startsWith('#')) return;
    const m = LINE.exec(line);
    if (m) names.add(m[1]);
    else bad.push(i + 1);
  });
  return { names, bad };
}

// Returns the variable names read in a Rust source text that the example does not list.
function missingFrom(names, source) {
  return [...source.matchAll(ENV_READ)].map((m) => m[1]).filter((n) => !names.has(n));
}

function selfTest() {
  const failures = [];
  if (!missingFrom(parseExample('A=\n').names, 'std::env::var("B")').includes('B')) {
    failures.push('self-test (a): unlisted variable B was not reported');
  }
  if (parseExample('A=secret\n').bad.length === 0) {
    failures.push('self-test (b): a line with a value was not flagged');
  }
  return failures;
}

function rustFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === 'target' ? [] : rustFiles(full);
    return entry.name.endsWith('.rs') ? [full] : [];
  });
}

const failures = selfTest();
if (failures.length) {
  console.log(failures.join('\n'));
  process.exit(1);
}

const example = parseExample(readFileSync(path.join(root, '.env.example'), 'utf8'));
if (example.bad.length) {
  console.log(`.env.example line(s) with a value or malformed: ${example.bad.join(', ')}`);
  process.exit(1);
}

let exit = 0;
for (const file of rustFiles(path.join(root, 'src'))) {
  const missing = missingFrom(example.names, readFileSync(file, 'utf8'));
  if (missing.length) {
    console.log(`missing from .env.example: ${[...new Set(missing)].join(', ')} (read in ${path.relative(root, file)})`);
    exit = 1;
  }
}
if (exit) process.exit(exit);

console.log(`env_example_ok vars=${example.names.size} self_test=pass`);
