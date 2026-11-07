// Builds aether-wasm with wasm-pack and checks that it runs in Node and reports the Cargo.toml version.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);

function expectedVersion() {
  const cargo = readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
  const block = cargo.split(/^\[/m).find((s) => s.startsWith('workspace.package]'));
  return block?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
}

try {
  // stdio inherited so the wasm-pack build output is visible.
  execFileSync('wasm-pack', ['build', path.join(root, 'src/wasm'), '--target', 'nodejs', '--dev'], {
    cwd: root,
    stdio: 'inherit',
  });
  const { version } = require(path.join(root, 'src/wasm/pkg/aether_wasm.js'));
  const reported = version();
  const expected = expectedVersion();
  if (reported !== expected) {
    console.log(`version mismatch: wasm reports ${reported}, Cargo.toml [workspace.package] says ${expected}`);
    process.exit(1);
  }
  console.log(`wasm_smoke_ok version=${reported}`);
} catch (err) {
  console.error(err);
  process.exit(1);
}
