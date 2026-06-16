import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

/**
 * Tests for the in-browser MCP bridge.
 *
 * The WASM dispatcher is stubbed — what we care about here is the JS
 * adapter: origin checks, parse handling, request routing, notification
 * handling, error envelopes, and the connect callback.
 */

const mcpRequest = vi.fn<(req: string) => string | undefined>();

// `server.ts` imports `getWasmModule` from `../bridge/backend`; stub it to
// return a fake module whose `mcp_request` is our spy.
vi.mock('../bridge/backend', () => ({
  getWasmModule: vi.fn(async () => ({ mcp_request: mcpRequest })),
  IS_WASM_BACKEND: true,
}));

import { LatticeMcpServer } from './server';

/**
 * Build a fake window with the addEventListener/removeEventListener pair
 * that `server.ts` uses, plus a way to fire `MessageEvent`s at it.
 */
function makeFakeWindow() {
  const listeners = new Map<string, Set<(e: unknown) => void>>();
  const win = {
    addEventListener(type: string, cb: (e: unknown) => void) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type)!.add(cb);
    },
    removeEventListener(type: string, cb: (e: unknown) => void) {
      listeners.get(type)?.delete(cb);
    },
    dispatch(type: string, event: unknown) {
      const set = listeners.get(type);
      if (!set) return;
      for (const cb of set) cb(event);
    },
    location: { origin: 'https://lattice.test' },
  };
  return win;
}

/**
 * Build a fake `MessageEvent`. `source.postMessage` is a spy so we can
 * inspect what the bridge sends back.
 */
function makeEvent(opts: { data: unknown; origin: string }) {
  const replies: Array<{ data: unknown; origin?: string }> = [];
  const source = {
    postMessage: vi.fn((data: unknown, origin?: string) => {
      replies.push({ data, origin });
    }),
  };
  return {
    event: { data: opts.data, origin: opts.origin, source } as unknown as MessageEvent,
    replies,
    sourceSpy: source.postMessage,
  };
}

beforeEach(() => {
  mcpRequest.mockReset();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('LatticeMcpServer — origin enforcement', () => {
  it('drops messages from non-allowed origins (no reply, onError fires)', async () => {
    const onError = vi.fn();
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
      onError,
    });
    server.attach();

    const { event, sourceSpy } = makeEvent({
      data: { jsonrpc: '2.0', id: 1, method: 'ping' },
      origin: 'https://evil.test',
    });
    await server.handleMessage(event);

    expect(mcpRequest).not.toHaveBeenCalled();
    expect(sourceSpy).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({ origin: 'https://evil.test', reason: 'origin-not-allowed' }),
    );
  });

  it('accepts messages from the allowlist', async () => {
    mcpRequest.mockReturnValueOnce(JSON.stringify({ jsonrpc: '2.0', id: 7, result: { ok: true } }));
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    server.attach();
    const { event, replies } = makeEvent({
      data: { jsonrpc: '2.0', id: 7, method: 'ping' },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(mcpRequest).toHaveBeenCalledTimes(1);
    expect(replies).toHaveLength(1);
    expect(replies[0].data).toEqual({ jsonrpc: '2.0', id: 7, result: { ok: true } });
    expect(replies[0].origin).toBe('https://lattice.test');
  });

  it('"*" allows any origin', async () => {
    mcpRequest.mockReturnValueOnce(JSON.stringify({ jsonrpc: '2.0', id: 1, result: {} }));
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: '*',
      targetWindow: win as unknown as Window,
    });
    const { event } = makeEvent({
      data: { jsonrpc: '2.0', id: 1, method: 'ping' },
      origin: 'https://someone-else.test',
    });
    await server.handleMessage(event);
    expect(mcpRequest).toHaveBeenCalledTimes(1);
  });
});

describe('LatticeMcpServer — request routing', () => {
  it('forwards a request as a JSON string to mcp_request and posts the parsed reply', async () => {
    mcpRequest.mockImplementationOnce((req) => {
      const parsed = JSON.parse(req);
      return JSON.stringify({ jsonrpc: '2.0', id: parsed.id, result: { echoed: parsed.method } });
    });
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    const { event, replies } = makeEvent({
      data: { jsonrpc: '2.0', id: 42, method: 'tools/list' },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(mcpRequest).toHaveBeenCalledWith(
      JSON.stringify({ jsonrpc: '2.0', id: 42, method: 'tools/list' }),
    );
    expect(replies[0].data).toEqual({ jsonrpc: '2.0', id: 42, result: { echoed: 'tools/list' } });
  });

  it('accepts the JSON payload either as a string or already-parsed object', async () => {
    mcpRequest.mockReturnValue(JSON.stringify({ jsonrpc: '2.0', id: 1, result: {} }));
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });

    // Object form
    const a = makeEvent({
      data: { jsonrpc: '2.0', id: 1, method: 'ping' },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(a.event);

    // String form
    const b = makeEvent({
      data: JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'ping' }),
      origin: 'https://lattice.test',
    });
    await server.handleMessage(b.event);

    expect(mcpRequest).toHaveBeenCalledTimes(2);
  });

  it('does not post anything for notifications (mcp_request returns undefined)', async () => {
    mcpRequest.mockReturnValueOnce(undefined);
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    const { event, sourceSpy } = makeEvent({
      data: { jsonrpc: '2.0', method: 'notifications/cancelled' }, // no id => notification
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(mcpRequest).toHaveBeenCalledTimes(1);
    expect(sourceSpy).not.toHaveBeenCalled();
  });

  it('ignores non-MCP envelopes silently (no engine call, no reply)', async () => {
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    const { event, sourceSpy } = makeEvent({
      data: { type: 'webpack-hmr', action: 'reload' },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(mcpRequest).not.toHaveBeenCalled();
    expect(sourceSpy).not.toHaveBeenCalled();
  });
});

describe('LatticeMcpServer — error handling', () => {
  it('replies with -32700 (parse error) for invalid JSON strings', async () => {
    const onError = vi.fn();
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
      onError,
    });
    const { event, replies } = makeEvent({
      data: '{not valid json',
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(replies[0].data).toEqual(
      expect.objectContaining({
        jsonrpc: '2.0',
        id: null,
        error: expect.objectContaining({ code: -32700 }),
      }),
    );
    expect(onError).toHaveBeenCalled();
  });

  it('wraps a thrown mcp_request as -32603 (internal error) preserving the id', async () => {
    mcpRequest.mockImplementationOnce(() => {
      throw new Error('engine exploded');
    });
    const onError = vi.fn();
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
      onError,
    });
    const { event, replies } = makeEvent({
      data: { jsonrpc: '2.0', id: 99, method: 'tools/call' },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(replies[0].data).toEqual(
      expect.objectContaining({
        jsonrpc: '2.0',
        id: 99,
        error: expect.objectContaining({ code: -32603, message: expect.stringContaining('engine exploded') }),
      }),
    );
    expect(onError).toHaveBeenCalled();
  });
});

describe('LatticeMcpServer — connection lifecycle', () => {
  it('fires onClientConnected exactly once on the first successful initialize', async () => {
    mcpRequest.mockReturnValue(
      JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        result: {
          protocolVersion: '2024-11-05',
          serverInfo: { name: 'lattice', version: 'test' },
        },
      }),
    );
    const onClientConnected = vi.fn();
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
      onClientConnected,
    });

    const a = makeEvent({
      data: { jsonrpc: '2.0', id: 1, method: 'initialize', params: { clientInfo: { name: 'demo' } } },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(a.event);
    // A second initialize should not re-fire the callback.
    const b = makeEvent({
      data: { jsonrpc: '2.0', id: 2, method: 'initialize', params: { clientInfo: { name: 'demo' } } },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(b.event);

    expect(onClientConnected).toHaveBeenCalledTimes(1);
    expect(onClientConnected).toHaveBeenCalledWith(
      expect.objectContaining({ origin: 'https://lattice.test', clientInfo: { name: 'demo' } }),
    );
    expect(server.isConnected).toBe(true);
  });

  it('does not flip isConnected when initialize returns an error', async () => {
    mcpRequest.mockReturnValue(
      JSON.stringify({ jsonrpc: '2.0', id: 1, error: { code: -32602, message: 'bad params' } }),
    );
    const onClientConnected = vi.fn();
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
      onClientConnected,
    });
    const { event } = makeEvent({
      data: { jsonrpc: '2.0', id: 1, method: 'initialize', params: {} },
      origin: 'https://lattice.test',
    });
    await server.handleMessage(event);
    expect(onClientConnected).not.toHaveBeenCalled();
    expect(server.isConnected).toBe(false);
  });

  it('attach/detach manage the window listener idempotently', () => {
    const win = makeFakeWindow();
    const addSpy = vi.spyOn(win, 'addEventListener');
    const removeSpy = vi.spyOn(win, 'removeEventListener');
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    server.attach();
    server.attach(); // idempotent
    expect(addSpy).toHaveBeenCalledTimes(1);
    server.detach();
    server.detach();
    expect(removeSpy).toHaveBeenCalledTimes(1);
  });

  it('dispatches via the window listener when attached (end-to-end through the listener)', async () => {
    mcpRequest.mockReturnValueOnce(JSON.stringify({ jsonrpc: '2.0', id: 11, result: 'ok' }));
    const win = makeFakeWindow();
    const server = new LatticeMcpServer({
      allowedOrigins: ['https://lattice.test'],
      targetWindow: win as unknown as Window,
    });
    server.attach();
    const { event, replies } = makeEvent({
      data: { jsonrpc: '2.0', id: 11, method: 'ping' },
      origin: 'https://lattice.test',
    });
    win.dispatch('message', event);
    // The listener fires the async handler — wait for it to settle.
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));
    expect(mcpRequest).toHaveBeenCalled();
    expect(replies).toHaveLength(1);
  });
});
