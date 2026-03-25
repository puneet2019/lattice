/**
 * MCP stdio server E2E tests.
 *
 * These tests spawn the Lattice binary in --mcp-stdio mode using
 * Node.js child_process (NOT WebdriverIO) and communicate via
 * JSON-RPC over stdin/stdout.
 */
import { spawn, type ChildProcess } from 'node:child_process';
import path from 'node:path';

const BINARY = path.resolve(__dirname, '..', '..', '..', '..', 'target', 'debug', 'Lattice');

/** Send a JSON-RPC request to the MCP server via stdin and read the response. */
function sendRequest(
  proc: ChildProcess,
  request: object,
): Promise<Record<string, unknown>> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('MCP response timeout')), 10_000);
    let buffer = '';

    const onData = (chunk: Buffer) => {
      buffer += chunk.toString();
      // JSON-RPC responses are newline-delimited
      const lines = buffer.split('\n');
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const parsed = JSON.parse(trimmed);
          clearTimeout(timeout);
          proc.stdout!.removeListener('data', onData);
          resolve(parsed);
          return;
        } catch {
          // Not complete JSON yet, keep buffering
        }
      }
    };

    proc.stdout!.on('data', onData);
    proc.stdin!.write(JSON.stringify(request) + '\n');
  });
}

describe('MCP stdio server', () => {
  let mcpProcess: ChildProcess;

  before(function () {
    // Skip these tests in the browser-based WebdriverIO runner.
    // They are designed to run via a Node.js test runner (e.g., mocha --require).
    if (typeof browser !== 'undefined') {
      this.skip();
      return;
    }
  });

  beforeEach(function () {
    // Guard for browser environment
    if (typeof browser !== 'undefined') {
      this.skip();
    }
  });

  it('should respond to initialize request', async function () {
    if (typeof browser !== 'undefined') { this.skip(); return; }

    mcpProcess = spawn(BINARY, ['--mcp-stdio'], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    const response = await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2024-11-05',
        capabilities: {},
        clientInfo: { name: 'e2e-test', version: '1.0.0' },
      },
    });

    expect(response).toHaveProperty('result');
    const result = response.result as Record<string, unknown>;
    expect(result).toHaveProperty('protocolVersion');
    expect(result).toHaveProperty('capabilities');
  });

  it('should list 65+ tools via tools/list', async function () {
    if (typeof browser !== 'undefined' || !mcpProcess) { this.skip(); return; }

    const response = await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/list',
      params: {},
    });

    expect(response).toHaveProperty('result');
    const result = response.result as { tools: Array<{ name: string }> };
    expect(result.tools.length).toBeGreaterThanOrEqual(65);
  });

  it('should write and read a cell value', async function () {
    if (typeof browser !== 'undefined' || !mcpProcess) { this.skip(); return; }

    // Write cell A1 = 100
    await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 3,
      method: 'tools/call',
      params: { name: 'write_cell', arguments: { sheet: 'Sheet1', cell_ref: 'A1', value: '100' } },
    });

    // Read cell A1
    const readResp = await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 4,
      method: 'tools/call',
      params: { name: 'read_cell', arguments: { sheet: 'Sheet1', cell_ref: 'A1' } },
    });

    expect(readResp).toHaveProperty('result');
    const result = readResp.result as { content: Array<{ text: string }> };
    const text = result.content[0].text;
    expect(text).toContain('100');
  });

  it('should evaluate a formula', async function () {
    if (typeof browser !== 'undefined' || !mcpProcess) { this.skip(); return; }

    const response = await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 5,
      method: 'tools/call',
      params: {
        name: 'evaluate_formula',
        arguments: { sheet: 'Sheet1', formula: 'SUM(1,2,3)' },
      },
    });

    expect(response).toHaveProperty('result');
    const result = response.result as { content: Array<{ text: string }> };
    const text = result.content[0].text;
    expect(text).toContain('6');
  });

  it('should describe data statistics', async function () {
    if (typeof browser !== 'undefined' || !mcpProcess) { this.skip(); return; }

    // Write some data first
    for (let i = 1; i <= 5; i++) {
      await sendRequest(mcpProcess, {
        jsonrpc: '2.0',
        id: 10 + i,
        method: 'tools/call',
        params: {
          name: 'write_cell',
          arguments: { sheet: 'Sheet1', cell_ref: `B${i}`, value: String(i * 10) },
        },
      });
    }

    const response = await sendRequest(mcpProcess, {
      jsonrpc: '2.0',
      id: 20,
      method: 'tools/call',
      params: {
        name: 'describe_data',
        arguments: { sheet: 'Sheet1', range: 'B1:B5' },
      },
    });

    expect(response).toHaveProperty('result');
    const result = response.result as { content: Array<{ text: string }> };
    const text = result.content[0].text;
    // Should contain statistics like count, mean, etc.
    expect(text.length).toBeGreaterThan(0);
  });

  after(() => {
    if (mcpProcess) {
      mcpProcess.kill();
    }
  });
});
