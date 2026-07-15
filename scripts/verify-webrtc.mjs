import { spawn, spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const nativeExecutable = join(root, 'beam-native', 'target', 'debug', 'eclipxse-beam-native.exe');
const browserExecutable = [
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
].find(existsSync);
const peerId = `eclipxseverify${Date.now()}`;
const previewPort = 4173;
const debugPort = 9237;
const temporaryRoot = mkdtempSync(join(tmpdir(), 'eclipxse-beam-webrtc-'));
const downloadRoot = join(temporaryRoot, 'downloads');
const browserProfile = join(temporaryRoot, 'edge-profile');
const fixture = join(temporaryRoot, `phone-proof-${Date.now()}.txt`);
const fixtureContents = `Eclipxse Beam WebRTC proof ${new Date().toISOString()}\n`;
const children = [];

function stopTree(child) {
  if (!child?.pid) return;
  spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
    stdio: 'ignore',
    windowsHide: true,
  });
}

async function waitFor(description, check, timeoutMilliseconds = 30_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const value = await check();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 250));
  }
  throw new Error(`${description} timed out${lastError ? `: ${lastError.message}` : ''}`);
}

class DevToolsClient {
  constructor(url) {
    this.nextId = 1;
    this.pending = new Map();
    this.socket = new WebSocket(url);
  }

  async open() {
    await new Promise((resolvePromise, rejectPromise) => {
      this.socket.addEventListener('open', resolvePromise, { once: true });
      this.socket.addEventListener('error', rejectPromise, { once: true });
    });
    this.socket.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      if (!message.id) return;
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    });
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return new Promise((resolvePromise, rejectPromise) => {
      this.pending.set(id, { resolve: resolvePromise, reject: rejectPromise });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  close() {
    this.socket.close();
  }
}

try {
  if (!existsSync(nativeExecutable)) throw new Error(`Native debug build is missing: ${nativeExecutable}`);
  if (!browserExecutable) throw new Error('Microsoft Edge or Google Chrome is required');
  mkdirSync(downloadRoot, { recursive: true });
  writeFileSync(fixture, fixtureContents);

  const native = spawn(nativeExecutable, [], {
    env: {
      ...process.env,
      ECLIPXSE_PEER_ID: peerId,
      ECLIPXSE_DOWNLOAD_ROOT: downloadRoot,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  children.push(native);

  const preview = spawn(
    process.execPath,
    [
      join(root, 'node_modules', 'vite', 'bin', 'vite.js'),
      'preview',
      '--host',
      '127.0.0.1',
      '--port',
      String(previewPort),
    ],
    { cwd: root, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true },
  );
  children.push(preview);
  await waitFor('Vite preview', async () => {
    const response = await fetch(`http://127.0.0.1:${previewPort}/`);
    return response.ok;
  });

  const companionUrl = `http://127.0.0.1:${previewPort}/?peer=${peerId}&native=1&auto=1`;
  const edge = spawn(
    browserExecutable,
    [
      '--headless=new',
      '--disable-gpu',
      '--no-first-run',
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${browserProfile}`,
      companionUrl,
    ],
    { stdio: 'ignore', windowsHide: true },
  );
  children.push(edge);

  const page = await waitFor('Edge DevTools page', async () => {
    const response = await fetch(`http://127.0.0.1:${debugPort}/json`);
    const pages = await response.json();
    return pages.find((candidate) => candidate.type === 'page' && candidate.url.includes(peerId));
  });
  const devtools = new DevToolsClient(page.webSocketDebuggerUrl);
  await devtools.open();
  await devtools.send('Runtime.enable');
  await devtools.send('DOM.enable');

  const status = await waitFor('WebRTC data channel', async () => {
    const result = await devtools.send('Runtime.evaluate', {
      expression: `document.querySelector('.status-chip')?.textContent?.trim() || ''`,
      returnByValue: true,
    });
    return result.result.value.includes('Connected') ? result.result.value : '';
  }, 45_000);

  const document = await devtools.send('DOM.getDocument', { depth: -1 });
  const input = await devtools.send('DOM.querySelector', {
    nodeId: document.root.nodeId,
    selector: 'input[type="file"]',
  });
  if (!input.nodeId) throw new Error('The phone file input was not found');
  await devtools.send('DOM.setFileInputFiles', {
    nodeId: input.nodeId,
    files: [fixture],
  });
  await waitFor('selected phone fixture', async () => {
    const result = await devtools.send('Runtime.evaluate', {
      expression: `document.querySelector('.selected-file strong')?.textContent || ''`,
      returnByValue: true,
    });
    return result.result.value.includes('phone-proof-');
  });
  await devtools.send('Runtime.evaluate', {
    expression: `document.querySelector('.beam-button')?.click()`,
  });

  const receivedPath = join(downloadRoot, fixture.split('\\').at(-1));
  await waitFor('native received file', () => existsSync(receivedPath), 45_000);
  const receivedContents = readFileSync(receivedPath, 'utf8');
  if (receivedContents !== fixtureContents) throw new Error('Received file contents did not match');

  console.log(JSON.stringify({
    status: 'passed',
    peerId,
    browserStatus: status,
    receivedFile: receivedPath,
    bytes: Buffer.byteLength(receivedContents),
  }));
  devtools.close();
} finally {
  for (const child of children.reverse()) stopTree(child);
  await new Promise((resolvePromise) => setTimeout(resolvePromise, 1_000));
  const resolvedTemporaryRoot = realpathSync(temporaryRoot);
  const resolvedSystemTemp = realpathSync(tmpdir());
  if (resolvedTemporaryRoot.startsWith(resolvedSystemTemp)) {
    rmSync(resolvedTemporaryRoot, {
      recursive: true,
      force: true,
      maxRetries: 10,
      retryDelay: 200,
    });
  }
}
