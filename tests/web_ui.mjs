// T15.G2: builds src/web, serves it with `vite preview`, runs tests/mock_stream.mjs, and drives headless Chrome against both.
// Checks: the point cloud and 6 Drones are drawn, LIVE and SIM labels are visible, a click on the Room Twin sends a Command
// whose Verdict appears in the HUD, and the map's frame rate over 5 s. Prints `web_ui_ok …` only if all hold and fps ≥ 30.
// Run from the repo root: node tests/web_ui.mjs   (screenshot: src/web/test-results/ui.png, gitignored)
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const web = path.join(root, 'src/web');
const requireWeb = createRequire(path.join(web, 'package.json'));
const puppeteer = requireWeb('puppeteer-core');
const viteBin = path.join(path.dirname(requireWeb.resolve('vite/package.json')), 'bin/vite.js');
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const VIEWPORT = { width: 1440, height: 900, deviceScaleFactor: 1 };
const headed = process.argv.includes('--headed');
const shotPath = path.join(web, `test-results/ui${headed ? '-headed' : ''}.png`);

const children = [];
let browser = null;

function freePort() {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.once('error', reject);
    s.listen(0, '127.0.0.1', () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
  });
}

function start(args, opts = {}) {
  const child = spawn(process.execPath, args, { stdio: ['ignore', 'pipe', 'pipe'], ...opts });
  children.push(child);
  child.log = '';
  child.stdout.on('data', (b) => (child.log += b));
  child.stderr.on('data', (b) => (child.log += b));
  return child;
}

async function waitFor(what, test, timeoutMs) {
  const until = Date.now() + timeoutMs;
  for (;;) {
    const v = await test();
    if (v) return v;
    if (Date.now() > until) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 100));
  }
}

async function cleanup() {
  try {
    await browser?.close();
  } catch {
    browser?.process()?.kill('SIGKILL');
  }
  for (const c of children) if (c.exitCode === null) c.kill('SIGTERM');
}

function fail(why, fields = '') {
  console.log(`web_ui_fail ${fields} reason=${JSON.stringify(why)}`.replace(/\s+/g, ' '));
  process.exitCode = 1;
}

async function main() {
  execFileSync(process.execPath, [path.join(root, 'tests/mock_stream.mjs'), '--self-test'], { stdio: 'pipe' });
  try {
    execFileSync('npm', ['run', 'build'], { cwd: web, stdio: 'pipe' });
  } catch (e) {
    throw new Error(`npm run build failed:\n${e.stdout}${e.stderr}`);
  }

  const mock = start([path.join(root, 'tests/mock_stream.mjs'), '--port', '0'], { cwd: root });
  const [, wsPort] = await waitFor('mock_stream', () => mock.log.match(/listening ws:\/\/127\.0\.0\.1:(\d+)\/ws/) || (mock.exitCode !== null && Promise.reject(new Error(mock.log))), 30000);

  const httpPort = await freePort();
  const preview = start([viteBin, 'preview', '--host', '127.0.0.1', '--port', String(httpPort), '--strictPort'], { cwd: web });
  const base = `http://127.0.0.1:${httpPort}/`;
  await waitFor('vite preview', () => fetch(base).then((r) => r.ok, () => preview.exitCode !== null && Promise.reject(new Error(preview.log))), 30000);

  browser = await puppeteer.launch({
    executablePath: CHROME,
    headless: !headed,
    timeout: 120000, // Chrome took over 30 s to start while other agents were compiling
    defaultViewport: VIEWPORT,
    args: ['--enable-gpu', '--ignore-gpu-blocklist', '--no-first-run', '--no-default-browser-check'],
  });
  const page = await browser.newPage();
  await page.bringToFront();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(`${base}?ws=${encodeURIComponent(`ws://127.0.0.1:${wsPort}/ws`)}`, { waitUntil: 'load', timeout: 60000 });

  await page.waitForFunction(() => window.__aether && window.__aether.points > 0 && window.__aether.drones === 6, { timeout: 30000 });
  const { points, drones, renderer } = await page.evaluate(() => ({ points: __aether.points, drones: __aether.drones, renderer: __aether.renderer }));
  console.error(`webgl renderer: ${renderer}${/swiftshader/i.test(renderer) ? ' (software fallback: fps is not the GPU figure)' : ''}`);

  // The Room Twin's tag must read LIVE and the six Swarm rows SIM; the static legend (which shows both) is not counted.
  await page.waitForFunction(() => document.querySelectorAll('.swarm li [data-provenance]').length === 6 && document.querySelector('.room [data-provenance]'), { timeout: 10000 });
  const labels = await page.evaluate(() => {
    const shown = (el) => {
      const cs = getComputedStyle(el);
      return el.getClientRects().length > 0 && cs.visibility !== 'hidden' && Number(cs.opacity) > 0;
    };
    const texts = (sel) => [...document.querySelectorAll(sel)].filter(shown).map((el) => el.textContent.trim());
    const room = texts('.room [data-provenance]'), swarm = texts('.swarm li [data-provenance]');
    return room.includes('LIVE') && swarm.length === 6 && swarm.every((t) => t === 'SIM') ? 1 : 0;
  });

  // Click floor spots away from the Launch pad (the Drones hover there) until one lands on the cloud.
  let verdict = 0, markerOnDown = false;
  for (const [n, e] of [[-1.5, -3], [1.5, -3.5], [-1.5, 3], [2, 2.5], [-3, 0]]) {
    const { x, y } = await page.evaluate((n, e) => __aether.project(n, e, 0), n, e);
    if (x < 400 || x > VIEWPORT.width - 420 || y < 160 || y > VIEWPORT.height - 140) continue; // clear of the HUD panels
    await page.mouse.move(x, y);
    await page.mouse.down();
    markerOnDown = await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(__aether.markerVisible))));
    await page.mouse.up();
    if (!markerOnDown) continue;
    verdict = await page
      .waitForFunction(() => __aether.lastVerdict?.approved === true && document.querySelector('.line[data-approved="true"]'), { timeout: 5000 })
      .then(() => 1, () => 0);
    if (verdict) break;
  }
  if (verdict && !/source=click kind=dispatch approved/.test(mock.log)) verdict = 0; // the engine side must have seen it

  async function measure(name) {
    await new Promise(r => setTimeout(r, 1500)); // finish resize/font/shader work before the steady-state sample
    if (headed) await page.tracing.start({ path: path.join(web, `test-results/${name}.trace.json`), categories: ['devtools.timeline', 'disabled-by-default-devtools.timeline.frame'] });
    const fps = await page.evaluate(
    () =>
      new Promise((r) => {
        const f0 = __aether.frames, t0 = performance.now();
        setTimeout(() => r(((__aether.frames - f0) * 1000) / (performance.now() - t0)), 5000);
      }),
    );
    if (headed) await page.tracing.stop();
    return fps;
  }
  mkdirSync(path.dirname(shotPath), { recursive: true });
  const fps = await measure('1440x900');
  await page.screenshot({ path: shotPath });
  // Reset is acknowledged by the server; it must clear local selection/follow and show approval.
  await page.click('.swarm .row');
  await page.keyboard.press('f');
  await page.keyboard.press('r');
  await page.waitForFunction(() => document.querySelector('.line[data-approved="true"]')?.textContent.includes('Reset:'), { timeout: 5000 });
  await page.waitForFunction(() => [...document.querySelectorAll('.swarm .row')].length === 6
    && [...document.querySelectorAll('.swarm .row')].every(el => el.getAttribute('aria-label').includes('disarmed')), { timeout: 5000 });
  const reset = await page.evaluate(() => [...document.querySelectorAll('[aria-pressed]')].every(el => el.getAttribute('aria-pressed') === 'false'));
  let externalFps = null;
  if (headed) {
    await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
    externalFps = await measure('1920x1080');
    await page.screenshot({ path: path.join(web, 'test-results/ui-external.png') });
  }

  const fields = `points=${points} drones=${drones} verdict=${verdict} labels=${labels} fps=${Math.floor(fps)}${headed ? ` external_fps=${Math.floor(externalFps)}` : ''} reset=${Number(reset)}`;
  if (errors.length) return fail(`page errors: ${errors.join('; ')}`, fields);
  if (points < 1000 || drones !== 6 || !verdict || !labels || !reset || fps < (headed ? 55 : 30) || (headed && externalFps < 55)) return fail(markerOnDown ? 'a check did not hold' : 'no click landed on the Room Twin', fields);
  console.log(`web_ui_ok ${fields}`);
}

for (const sig of ['SIGINT', 'SIGTERM']) process.on(sig, () => cleanup().then(() => process.exit(1)));
main()
  .catch((e) => fail(e.message))
  .finally(cleanup);
