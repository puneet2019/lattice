#!/usr/bin/env node
/**
 * End-to-end smoke test for the in-browser MCP bridge.
 *
 * Pre-requisites (the Makefile `mcp-verify` target wires these up):
 *   1. `make web` has built `frontend/dist/` with the WASM bundle.
 *   2. Playwright is installed at `frontend/node_modules/playwright`
 *      (kept off package.json deliberately — installed via the make
 *      target with `--no-save --no-package-lock`).
 *
 * What it does:
 *   - Starts a tiny static server over `frontend/dist/`.
 *   - Launches system Chrome via Playwright.
 *   - Loads `http://localhost:<port>/mcp-demo.html`.
 *   - Drives the demo's postMessage flow from inside the page context:
 *       a) initialize        → asserts serverInfo.name === 'lattice'.
 *       b) tools/list        → asserts >= 30 tools (the real catalog is much larger).
 *       c) tools/call write_cell + read_cell → asserts the round-trip.
 *
 * Exits 0 on success, non-zero with a descriptive message on failure.
 */
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const DIST_ROOT = join(REPO_ROOT, 'frontend', 'dist');

const require = createRequire(import.meta.url);
// Absolute path: avoids any chance of a stray top-level `playwright` resolve.
const playwrightPath = join(REPO_ROOT, 'frontend', 'node_modules', 'playwright');
const { chromium } = require(playwrightPath);

// ---------- tiny static server ---------------------------------------------

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
  '.ico': 'image/x-icon',
  '.png': 'image/png',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

function startStaticServer(root) {
  const server = createServer(async (req, res) => {
    try {
      let urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
      if (urlPath === '/' || urlPath === '') urlPath = '/index.html';
      // Naive path safety — strip any traversal.
      urlPath = urlPath.replace(/\.\.+/g, '.');
      const filePath = join(root, urlPath);
      const data = await readFile(filePath);
      const ext = filePath.slice(filePath.lastIndexOf('.'));
      res.writeHead(200, {
        'Content-Type': MIME[ext] || 'application/octet-stream',
        // Required for some WASM features and clean iframe behavior.
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
        'Cross-Origin-Resource-Policy': 'same-origin',
      });
      res.end(data);
    } catch (e) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end(`404: ${e.message}`);
    }
  });
  return new Promise((resolveFn) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolveFn({ server, port });
    });
  });
}

// ---------- main -----------------------------------------------------------

async function main() {
  const { server, port } = await startStaticServer(DIST_ROOT);
  const url = `http://127.0.0.1:${port}/mcp-demo.html`;
  console.log(`[mcp-verify] static server on ${url}`);

  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  page.on('pageerror', (err) => console.error('[pageerror]', err.message));
  page.on('console', (msg) => {
    if (msg.type() === 'error') console.error('[console.error]', msg.text());
  });

  try {
    await page.goto(url, { waitUntil: 'load' });

    // Drive the demo from inside the parent page context — bypasses the UI
    // and goes straight at `iframe.contentWindow.postMessage`.
    const result = await page.evaluate(async () => {
      /** @type {HTMLIFrameElement} */
      const iframe = document.getElementById('lattice');
      // Wait for the iframe to have loaded the SPA + WASM. Give it generous time.
      await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('iframe never loaded')), 30000);
        if (iframe.contentDocument && iframe.contentWindow) {
          // Already loaded — but still give the WASM a beat.
          setTimeout(() => { clearTimeout(timeout); resolve(); }, 2000);
        } else {
          iframe.addEventListener('load', () => {
            setTimeout(() => { clearTimeout(timeout); resolve(); }, 2000);
          });
        }
      });

      const pending = new Map();
      let nextId = 1;
      window.addEventListener('message', (e) => {
        if (e.source !== iframe.contentWindow) return;
        const data = e.data;
        if (!data || typeof data !== 'object' || data.jsonrpc !== '2.0') return;
        if (data.id != null && pending.has(data.id)) {
          const { resolve, reject } = pending.get(data.id);
          pending.delete(data.id);
          if (data.error) reject(new Error(data.error.message || 'mcp error'));
          else resolve(data.result);
        }
      });

      function call(method, params) {
        const id = nextId++;
        const msg = { jsonrpc: '2.0', id, method, params };
        return new Promise((resolve, reject) => {
          pending.set(id, { resolve, reject });
          setTimeout(() => {
            if (pending.has(id)) {
              pending.delete(id);
              reject(new Error(`Timeout waiting for ${method}`));
            }
          }, 20000);
          iframe.contentWindow.postMessage(msg, location.origin);
        });
      }

      // Retry initialize a few times — WASM init can race the first message.
      let initResult;
      let lastErr;
      for (let i = 0; i < 10; i++) {
        try {
          initResult = await call('initialize', {
            protocolVersion: '2024-11-05',
            capabilities: {},
            clientInfo: { name: 'e2e', version: '0.1.0' },
          });
          break;
        } catch (e) {
          lastErr = e;
          await new Promise((r) => setTimeout(r, 1000));
        }
      }
      if (!initResult) throw lastErr || new Error('initialize failed');

      const toolsResult = await call('tools/list', {});

      // First write, then read back from the same cell.
      const writeResult = await call('tools/call', {
        name: 'write_cell',
        arguments: { sheet: 'Sheet1', cell_ref: 'A1', value: 'mcp-e2e' },
      });

      const readResult = await call('tools/call', {
        name: 'read_cell',
        arguments: { sheet: 'Sheet1', cell_ref: 'A1' },
      });

      return { initResult, toolsResult, writeResult, readResult };
    });

    // ---- assertions --------------------------------------------------------
    const { initResult, toolsResult, writeResult, readResult } = result;
    const failures = [];

    const serverName = initResult?.serverInfo?.name;
    if (serverName !== 'lattice') {
      failures.push(`expected serverInfo.name === 'lattice', got ${JSON.stringify(serverName)}`);
    }
    const proto = initResult?.protocolVersion;
    if (typeof proto !== 'string' || proto.length === 0) {
      failures.push(`expected non-empty protocolVersion, got ${JSON.stringify(proto)}`);
    }

    const tools = toolsResult?.tools;
    if (!Array.isArray(tools)) {
      failures.push(`expected tools/list result.tools to be an array, got ${typeof tools}`);
    } else if (tools.length < 30) {
      failures.push(`expected >= 30 tools, got ${tools.length}`);
    }

    // The MCP tool result envelope is `{ content: [{ type, text }], isError? }`.
    // For read_cell, the text payload should mention or contain the value we wrote.
    const readText = (() => {
      if (!readResult) return null;
      if (typeof readResult === 'string') return readResult;
      const content = readResult.content;
      if (Array.isArray(content) && content[0]?.text) return content[0].text;
      try { return JSON.stringify(readResult); } catch { return String(readResult); }
    })();
    if (!readText || !readText.includes('mcp-e2e')) {
      failures.push(
        `round-trip read_cell did not return the value written. read=${JSON.stringify(readResult)} write=${JSON.stringify(writeResult)}`,
      );
    }

    if (failures.length > 0) {
      console.error('[mcp-verify] FAIL');
      for (const f of failures) console.error('  -', f);
      throw new Error(`${failures.length} assertion(s) failed`);
    }

    console.log(`[mcp-verify] PASS — initialize OK, tools=${tools.length}, round-trip OK`);
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((e) => {
  console.error('[mcp-verify] error:', e?.stack || e?.message || e);
  process.exit(1);
});
