//! MCP (Model Context Protocol) server for Lattice.
//!
//! Implements JSON-RPC 2.0 dispatch as a sync function over a borrowed
//! [`McpState`], so the same dispatch code runs in two places:
//!
//! - **Native (default).** The desktop build enables the `native`
//!   feature, which pulls in tokio + hyper transports. [`McpServer`]
//!   wraps the workbook in `Arc<RwLock<...>>` and exposes
//!   `run_stdio` / `run_http`. Internally those transports call
//!   [`handle_request`] after acquiring locks.
//! - **WASM (`--no-default-features`).** The browser build skips the
//!   transports and the tokio wrapper, exposing only [`McpState`] and
//!   [`handle_request`]. `lattice-wasm` drives the protocol over
//!   postMessage by handing each incoming JSON-RPC string directly to
//!   `handle_request`.

pub mod dispatch;
pub mod prompts;
pub mod resources;
pub mod schema;
pub mod tools;

// `McpServer` (the tokio/Arc<RwLock> wrapper) and the stdio + HTTP
// transports require tokio + hyper. They are gated behind `native` so
// `--no-default-features` builds — most importantly the wasm32 target —
// don't pull them in.
#[cfg(feature = "native")]
pub mod server;
#[cfg(feature = "native")]
pub mod transport;

#[cfg(feature = "native")]
pub use server::McpServer;

// The sync dispatcher is the WASM-facing entry point. Re-export it at the
// crate root so callers can `use lattice_mcp::{McpState, handle_request};`
// without reaching into the `dispatch` module.
pub use dispatch::{McpState, PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION, handle_request};
