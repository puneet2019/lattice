# Lattice — MCP Integration References

> For agents to consult when implementing the MCP server.
> Real-world examples of MCP servers with patterns worth copying.

---

## 1. MCP Protocol Quick Reference

**Protocol version**: `2025-11-25` (latest stable)
**Base protocol**: JSON-RPC 2.0, stateful connections

### Transport Types

| Transport | Use Case | How It Works |
|-----------|----------|-------------|
| **stdio** | Local apps (Claude Desktop/Code) | JSON-RPC over stdin/stdout. No network. Best for desktop apps. |
| **Streamable HTTP** | Networked/remote agents | HTTP POST for requests, optional SSE for server push. Supports OAuth. |

Note: SSE-only transport is deprecated in favor of Streamable HTTP.

### Server Primitives

- **Tools** — Functions the AI can call. Defined by `name`, `title`, `description`, `inputSchema` (JSON Schema). Invoked via `tools/call`.
- **Resources** — Data the AI can read. Identified by URI. Read via `resources/read`.
- **Prompts** — Reusable message templates with typed arguments. Retrieved via `prompts/get`.

### Lifecycle

```
1. Client → initialize (protocolVersion, capabilities, clientInfo)
2. Server → response (capabilities, serverInfo)
3. Client → notifications/initialized
4. Client → tools/list, resources/list, prompts/list
5. Client → tools/call (tool invocations)
6. Server → notifications/tools/list_changed (when tools change dynamically)
```

---

## 2. Rust MCP SDK

**Repo**: https://github.com/modelcontextprotocol/rust-sdk
**Crates**: `rmcp` (core), `rmcp-macros` (proc macros)

### Key Features
- Feature-gated transports: `transport-async-rw` (stdio), `transport-streamable-http-server` (HTTP)
- `schemars` integration — `#[derive(JsonSchema)]` on tool arg structs auto-generates `inputSchema`
- `#[tool]` macro on async functions for tool definition
- `ServerHandler` trait for manual control
- WASI/WASM compatible

### Usage Example

```rust
use rmcp::{Server, ServerHandler, tool};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct ReadCellArgs {
    /// Sheet name (defaults to active sheet)
    sheet: Option<String>,
    /// Cell reference (e.g., "A1", "B5")
    cell: String,
}

#[tool(description = "Read the value and formula of a cell")]
async fn read_cell(args: ReadCellArgs) -> Result<String, McpError> {
    // implementation
}
```

### Transport Feature Flags (for lean builds)
```toml
[dependencies]
rmcp = { version = "0.1", features = ["transport-async-rw"] }  # stdio only
# Add "transport-streamable-http-server" for HTTP transport
```

---

## 3. Official Reference Servers (study these implementations)

### Filesystem Server — ACCESS SCOPING PATTERN
- **Repo**: https://github.com/modelcontextprotocol/servers (filesystem package)
- **Transport**: stdio
- **14 tools**: `read_text_file`, `write_file`, `edit_file`, `list_directory`, `search_files`, etc.
- **Key pattern**: Access gated to directories specified at startup or via Roots protocol. No arbitrary filesystem traversal.
- **Copy this for Lattice**: Scope MCP file access to explicitly opened files or approved paths only.

### Git Server — STATELESS TOOL DESIGN
- **Repo**: https://github.com/modelcontextprotocol/servers (git package)
- **12 tools**: `git_status`, `git_diff`, `git_commit`, `git_add`, etc.
- **Key pattern**: Every tool takes an explicit `repo_path` parameter — stateless between calls.
- **Copy this for Lattice**: Pass `sheet` name explicitly in every tool call rather than maintaining session state.

### Memory Server — STRUCTURED DATA STORE
- **Repo**: https://github.com/modelcontextprotocol/servers (memory package)
- **9 tools**: `create_entities`, `create_relations`, `add_observations`, `read_graph`, `search_nodes`
- **Key pattern**: Exposes a structured data store with atomic CRUD operations.
- **Copy this for Lattice**: Model cells/ranges as entities with atomic operations.

### Fetch Server — CHUNKED DELIVERY
- **1 tool**: `fetch` with `max_length`, `start_index`, `raw` params
- **Key pattern**: Returns partial results, lets the model call again with `start_index` for pagination.
- **Copy this for Lattice**: For large sheet reads, support `start_row`/`end_row` parameters and return a `nextCursor`.

### Everything Server — PROTOCOL TEST HARNESS
- **17 tools, 4 prompts, dynamic/static/session-scoped resources**
- Exercises every MCP feature: sampling, elicitation, progress notifications, structured content
- **Use this for Lattice**: Test your MCP implementation against every protocol feature.

---

## 4. Production MCP Servers (real-world patterns)

### GitHub MCP Server — DUAL TRANSPORT + DYNAMIC TOOLS
- **Repo**: https://github.com/github/github-mcp-server
- **Transport**: stdio (local) AND Streamable HTTP (remote at `https://api.githubcopilot.com/mcp/`)
- **Toolsets**: repos, issues, pull_requests, actions, code_security, discussions
- **Key patterns**:
  1. **Dual transport** in one binary — `--mode stdio` vs `--mode http`
  2. **Selectable toolsets** via `--tools` flag — server exposes only relevant tools
  3. **`notifications/tools/list_changed`** when context changes
- **Copy this for Lattice**: `lattice --mcp-stdio` vs `lattice --mcp-port 3141`. Expose only tools relevant to open workbook.

### Sentry MCP Server — HOSTED HTTP WITH OAUTH
- **URL**: `https://mcp.sentry.io` (hosted)
- **Transport**: Streamable HTTP with SSE fallback; stdio for self-hosted
- **16+ tools**: organizations, projects, issues, error search, AI fix suggestions
- **Key pattern**: Canonical hosted remote MCP server with OAuth authentication.
- **Relevant for Lattice Phase 4**: If we add remote/cloud features.

### Stripe MCP Server — CLEAN DUAL MODE
- **URL**: `https://mcp.stripe.com` (hosted); `npx @stripe/mcp` (local)
- **Transport**: Streamable HTTP (remote), stdio (local)
- **Key pattern**: Local mode uses personal API key, no OAuth needed. Clean separation.
- **Copy this for Lattice**: Local stdio = no auth. HTTP = optional auth.

### Supabase MCP Server — READ-ONLY MODE
- **Repo**: https://github.com/supabase-community/supabase-mcp
- **Transport**: Streamable HTTP
- **Key patterns**:
  1. **Read-only mode** — restricts to read-only operations
  2. **Project scoping** — tools operate on specified project only
  3. **Feature group filtering** — expose subsets of tools
- **Copy this for Lattice**: `--mcp-readonly` flag. Scope to open workbook.

### Playwright MCP — DESKTOP APP GOLD STANDARD
- **Repo**: https://github.com/microsoft/playwright-mcp
- **Transport**: stdio (default) and SSE/Streamable HTTP (standalone)
- **Key patterns**:
  1. **Persistent state** across tool calls (browser session stays open)
  2. **Two transport modes** in one binary
  3. **Security controls**: `--allowed-hosts`, `--blocked-origins`
  4. **Structured data** representations (accessibility tree, not raw pixels)
- **This is the closest model to what Lattice needs**: A desktop app exposing its state via MCP.

---

## 5. Database MCP Servers (tabular data — closest to spreadsheets)

### DBHub — CUSTOM TOOL INJECTION
- **Repo**: https://github.com/bytebase/dbhub
- **Transport**: stdio and HTTP
- **Databases**: PostgreSQL, MySQL, SQLite, SQL Server
- **Tools**: `execute_sql`, `search_objects`
- **Key pattern**: Custom parameterized SQL tools defined in `dbhub.toml` config, injected as MCP tools at startup.
- **Copy this for Lattice Phase 4**: Let users define named query tools (e.g., "get_portfolio_value" → runs specific formula/range query).

### SQLite Explorer — SAFETY-FIRST
- **Repo**: https://github.com/hannesrudolph/sqlite-explorer-fastmcp-mcp-server
- **Tools**: `read_query` (SELECT only), `list_tables`, `describe_table`
- **Key pattern**: Safety-first — SELECT only, row limits, parameter binding, input sanitization.
- **Copy this for Lattice**: In read-only mode, only allow read_cell/read_range, no mutations.

---

## 6. Design Patterns to Copy for Lattice MCP Server

### Pattern 1: Access Scoping (from Filesystem server)
```
Gate all tools to specific files/workbooks:
- stdio mode: operates on file passed via --file flag or currently open in GUI
- Never allow arbitrary filesystem traversal
- MCP open_file validates path before loading
```

### Pattern 2: Stateless Tools (from Git server)
```
Every tool takes explicit sheet/cell parameters:
- read_cell({ sheet: "Holdings", cell: "A1" })
- Never rely on "current sheet" state in MCP (that's GUI state)
- If sheet is omitted, default to first sheet
```

### Pattern 3: Read-Only Mode (from Supabase server)
```
--mcp-readonly flag:
- Only expose: read_cell, read_range, list_sheets, get_workbook_info, describe_data
- Block: write_cell, write_range, create_sheet, delete_sheet, etc.
- Useful for shared/public workbooks
```

### Pattern 4: Chunked Range Reads (from Fetch server)
```
For large sheets, don't return all data at once:
- read_range({ sheet: "Data", range: "A1:Z1000", max_rows: 100 })
- Return { data: [...], hasMore: true, nextCursor: "A101:Z200" }
- Let the AI paginate through large datasets
```

### Pattern 5: Dual Transport (from GitHub, Stripe, Playwright)
```
One binary, two modes:
- lattice --mcp-stdio          → stdio transport for Claude Desktop/Code
- lattice --mcp-port 3141      → Streamable HTTP for networked agents
- Both share the same tool/resource implementations
- Transport layer is abstracted behind a trait
```

### Pattern 6: Live State Sync (from Playwright)
```
When GUI is running and MCP is connected:
- MCP mutations → emit event → GUI re-renders
- GUI changes → MCP can optionally subscribe to notifications
- Shared state via Arc<RwLock<Workbook>>
```

### Pattern 7: Structured Content (from Playwright)
```
Return typed JSON, not raw strings:
read_cell returns:
{
  "cell": "A1",
  "value": 42.5,
  "type": "number",
  "formula": "=SUM(B1:B10)",
  "format": "currency",
  "display": "$42.50"
}
Not just: "42.5"
```

### Pattern 8: Dynamic Tool Registration (from GitHub)
```
When workbook changes, update available tools:
- Workbook with financial data → expose portfolio_summary tool
- Empty workbook → hide analysis tools
- Send notifications/tools/list_changed to connected clients
```

---

## 7. MCP Specification

Full specification: https://modelcontextprotocol.io/specification

Key sections to study:
- **Protocol lifecycle**: https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle
- **Tools**: https://modelcontextprotocol.io/specification/2025-11-25/server/tools
- **Resources**: https://modelcontextprotocol.io/specification/2025-11-25/server/resources
- **Prompts**: https://modelcontextprotocol.io/specification/2025-11-25/server/prompts
- **Transports**: https://modelcontextprotocol.io/specification/2025-11-25/basic/transports
