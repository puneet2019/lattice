/**
 * In-browser MCP bridge.
 *
 * A `LatticeMcpServer` listens for `postMessage` events from a parent window
 * (the ChatGPT Apps SDK shell, Claude.ai's MCP host, Arc Max, …), validates
 * the sender origin, forwards each MCP JSON-RPC 2.0 request to the WASM
 * dispatcher via `mcp_request(...)`, and posts the response back to the
 * sender — the WASM crate is the **single source of truth** for the tool
 * catalog, error codes, and protocol version. This file is just plumbing.
 *
 * The bridge defaults to **same-origin only**. Wider allowlists are opt-in.
 * Unauthorized messages are silently dropped (the demo / debug log will
 * surface them so a developer can spot a misconfiguration).
 */
import { getWasmModule } from '../bridge/backend';
import { isOriginAllowed, type AllowedOrigins } from './origin';

/** A parsed JSON-RPC 2.0 envelope (id is optional → notification). */
export interface JsonRpcMessage {
  jsonrpc: '2.0';
  id?: number | string | null;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: unknown;
}

export interface LatticeMcpServerOptions {
  /**
   * Origins allowed to send MCP messages. Defaults to `[window.location.origin]`
   * (same-origin only). Pass `"*"` to accept any origin — embedders must
   * explicitly opt in to this.
   */
  allowedOrigins?: AllowedOrigins;

  /**
   * Window that hosts the listener. Defaults to `globalThis.window`.
   * Override for tests.
   */
  targetWindow?: Window;

  /** Log every accepted / rejected message to the console. */
  debug?: boolean;

  /**
   * Fires once after any client successfully completes an `initialize`
   * handshake. Use it to flip a "MCP connected" indicator in the UI.
   */
  onClientConnected?: (info: { origin: string; clientInfo?: unknown }) => void;

  /** Fires for every accepted request after it is dispatched. */
  onRequest?: (info: { origin: string; method: string; id: unknown }) => void;

  /** Fires when a request is rejected (bad origin, parse error, dispatch throws). */
  onError?: (info: { origin: string; reason: string; raw?: unknown }) => void;
}

/**
 * Minimal `postMessage`-capable type for the sender. `MessageEvent.source`
 * is typed as `Window | MessagePort | ServiceWorker`, all of which accept
 * `postMessage` with a different signature — we only ever need the data +
 * target-origin shape, so we type-narrow ourselves.
 */
type MessageSender = {
  postMessage: (data: unknown, targetOrigin?: string) => void;
};

/**
 * Bridge that adapts browser `postMessage` traffic onto the WASM MCP
 * dispatcher. One instance handles many concurrent clients — each request
 * is dispatched independently and the response is posted back to the
 * specific `event.source` it came from.
 */
export class LatticeMcpServer {
  private readonly allowedOrigins: AllowedOrigins;
  private readonly targetWindow: Window;
  private readonly debug: boolean;
  private readonly opts: LatticeMcpServerOptions;
  private listener: ((event: MessageEvent) => void) | null = null;
  private hasNotifiedConnect = false;

  constructor(opts: LatticeMcpServerOptions = {}) {
    this.opts = opts;
    this.allowedOrigins =
      opts.allowedOrigins ??
      // Lazy default; if there's no window at construction time (SSR?), the
      // attach() call will throw with a clearer message instead.
      (typeof window !== 'undefined' ? [window.location.origin] : []);
    this.targetWindow =
      opts.targetWindow ?? (typeof window !== 'undefined' ? window : (undefined as unknown as Window));
    this.debug = !!opts.debug;
  }

  /** Whether at least one client has completed `initialize`. */
  get isConnected(): boolean {
    return this.hasNotifiedConnect;
  }

  /** Install the `message` listener. Idempotent. */
  attach(): void {
    if (this.listener) return;
    if (!this.targetWindow || typeof this.targetWindow.addEventListener !== 'function') {
      throw new Error('LatticeMcpServer: no window available to attach to');
    }
    this.listener = (event: MessageEvent) => {
      void this.handleMessage(event);
    };
    this.targetWindow.addEventListener('message', this.listener);
    if (this.debug) {
      // eslint-disable-next-line no-console
      console.log('[mcp] attached; allowedOrigins =', this.allowedOrigins);
    }
  }

  /** Remove the `message` listener. Safe to call repeatedly. */
  detach(): void {
    if (!this.listener) return;
    this.targetWindow.removeEventListener('message', this.listener);
    this.listener = null;
    if (this.debug) {
      // eslint-disable-next-line no-console
      console.log('[mcp] detached');
    }
  }

  /**
   * Dispatch one incoming `MessageEvent`. Public for tests; production code
   * goes through `attach()` and the registered listener.
   */
  async handleMessage(event: MessageEvent): Promise<void> {
    const origin = event.origin;
    if (!isOriginAllowed(origin, this.allowedOrigins)) {
      this.opts.onError?.({ origin, reason: 'origin-not-allowed', raw: event.data });
      if (this.debug) {
        // eslint-disable-next-line no-console
        console.warn('[mcp] dropped message from disallowed origin:', origin);
      }
      return;
    }

    // Embedders post either a JSON string or an already-parsed object —
    // accept both, normalize to an object.
    let message: JsonRpcMessage | null = null;
    try {
      message = typeof event.data === 'string' ? JSON.parse(event.data) : (event.data as JsonRpcMessage);
    } catch (e) {
      this.opts.onError?.({ origin, reason: `parse-error: ${(e as Error).message}`, raw: event.data });
      // Per JSON-RPC: a parse error has no id. Reply if we have a source.
      this.replyError(event.source, origin, null, -32700, 'Parse error');
      return;
    }

    if (!message || typeof message !== 'object' || message.jsonrpc !== '2.0') {
      // Not an MCP envelope — ignore silently. Other libraries also use
      // postMessage and we don't want to spam errors back at them.
      return;
    }

    // Dispatch through the WASM bridge.
    const requestStr = JSON.stringify(message);
    let responseStr: string | undefined;
    try {
      const mod = await getWasmModule();
      responseStr = mod.mcp_request(requestStr);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      this.opts.onError?.({ origin, reason: `dispatch-error: ${msg}`, raw: message });
      // Internal error envelope. Only reply if this was a request (has id).
      const id = message.id ?? null;
      if (message.method && id !== undefined) {
        this.replyError(event.source, origin, id, -32603, `Internal error: ${msg}`);
      }
      return;
    }

    if (message.method) {
      this.opts.onRequest?.({ origin, method: message.method, id: message.id ?? null });
    }

    // Notifications (no id) yield no response — the WASM side returns
    // undefined and we stay silent.
    if (responseStr === undefined) return;

    let response: JsonRpcMessage;
    try {
      response = JSON.parse(responseStr) as JsonRpcMessage;
    } catch {
      // Should never happen — the Rust side serializes its own envelopes.
      this.opts.onError?.({ origin, reason: 'engine-returned-bad-json', raw: responseStr });
      return;
    }

    this.postReply(event.source, origin, response);

    // Notify-on-first-initialize so the UI can flip a connected indicator.
    if (!this.hasNotifiedConnect && message.method === 'initialize' && !this.isErrorResponse(response)) {
      this.hasNotifiedConnect = true;
      const clientInfo = this.extractClientInfo(message.params);
      this.opts.onClientConnected?.({ origin, clientInfo });
    }
  }

  // ---- helpers ----------------------------------------------------------

  /** Build and send a JSON-RPC error envelope (best-effort). */
  private replyError(
    source: MessageEventSource | null,
    origin: string,
    id: number | string | null,
    code: number,
    message: string,
  ): void {
    const envelope: JsonRpcMessage = {
      jsonrpc: '2.0',
      id,
      error: { code, message },
    };
    this.postReply(source, origin, envelope);
  }

  /** Post a reply to the sender, narrowing the messy `MessageEventSource` union. */
  private postReply(source: MessageEventSource | null, origin: string, payload: unknown): void {
    if (!source) return;
    const sender = source as unknown as MessageSender;
    try {
      // Some senders (MessagePort, ServiceWorker) don't accept a targetOrigin.
      // We try-with-origin first, then fall back to a single-arg call.
      sender.postMessage(payload, origin);
    } catch {
      try {
        sender.postMessage(payload);
      } catch {
        // Give up silently — the sender is gone (closed tab, detached port).
      }
    }
  }

  private isErrorResponse(msg: JsonRpcMessage): boolean {
    return msg.error !== undefined && msg.error !== null;
  }

  private extractClientInfo(params: unknown): unknown {
    if (params && typeof params === 'object' && 'clientInfo' in params) {
      return (params as { clientInfo?: unknown }).clientInfo;
    }
    return undefined;
  }
}
