/**
 * In-browser MCP bridge — public entry point.
 *
 * The bridge lets a parent window (ChatGPT Apps SDK, Claude.ai, Arc Max …)
 * drive the live Lattice web app by exchanging MCP JSON-RPC 2.0 messages
 * over `postMessage`. The Rust/WASM side does the actual dispatch — this
 * package is the thin, security-aware JS adapter.
 */
export { LatticeMcpServer } from './server';
export type { LatticeMcpServerOptions, JsonRpcMessage } from './server';
export { isOriginAllowed, parseOriginList } from './origin';
export type { AllowedOrigins } from './origin';
