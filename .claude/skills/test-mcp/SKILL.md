---
name: test-mcp
description: Test the MCP server — run MCP integration tests and validate protocol compliance
---

# /test-mcp

Test the Lattice MCP server.

## Steps

1. Run MCP-specific tests:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo test -p lattice-mcp 2>&1
   ```

2. Run MCP integration tests:
   ```bash
   cargo test --test mcp_tests 2>&1
   ```

3. If MCP server binary exists, do a live smoke test:
   ```bash
   # Send initialize message via stdio
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' | cargo run -- --mcp-stdio 2>/dev/null | head -1
   ```

4. Report: tests passed/failed, protocol compliance issues, tool coverage.

## What to Validate
- JSON-RPC 2.0 message format
- initialize/initialized lifecycle
- tools/list returns all registered tools with valid JSON Schema
- resources/list returns all resources
- Each tool handles invalid inputs gracefully (returns error, doesn't panic)
- stdio transport reads/writes correctly
- HTTP transport responds on configured port
