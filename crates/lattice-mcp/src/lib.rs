//! MCP (Model Context Protocol) server for Lattice.
//!
//! Implements JSON-RPC 2.0 over stdio and HTTP transports,
//! exposing spreadsheet operations as MCP tools.

pub mod prompts;
pub mod resources;
pub mod schema;
pub mod server;
pub mod tools;
pub mod transport;

pub use server::McpServer;
