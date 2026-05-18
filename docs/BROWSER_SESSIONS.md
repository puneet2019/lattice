# In-Browser MCP Sessions

> Plan for running Lattice where the AI client lives in the browser — Claude.ai, ChatGPT, Arc Max, Dia, Comet — instead of as a desktop app shelling out to `lattice --mcp-stdio`.
>
> This subsumes the "MCP streamable HTTP transport" item deferred from v0.1.0 in `PLAN.md §What's Next`.

## Why now

Three browser-hosted MCP environments have shipped since the v0.1.0 release:

- **ChatGPT Apps SDK** — MCP servers register as Apps; users add them in Settings; ChatGPT can call tools inline in chat and render `text/html` UI returned by tools.
- **Claude.ai web** — Remote MCP server support (Streamable HTTP) for paid users; OAuth 2.1 discovery via `.well-known/oauth-protected-resource`.
- **AI-native browsers** — Arc Max, Dia, Comet, Brave Leo embed assistants that consume MCP servers the user configures.

Lattice today is stdio-only. That confines it to local clients (Claude Desktop, Claude Code, Cursor, Zed). Exposing `https://<host>/mcp` over Streamable HTTP makes every one of the above clients a Lattice driver — no `.dmg` install required.

## Where we already are

`crates/lattice-mcp/src/transport/http.rs` is a 281-line working draft:

- `POST /mcp` accepts JSON-RPC 2.0 → returns JSON-RPC response
- `GET /mcp/sse` for server→client notifications (broadcast channel)
- `GET /health`, CORS preflight handler, 5 tests
- Wired into `src-tauri/src/main.rs` behind `--mcp-http [port]`
- Binds `127.0.0.1` only — no auth, no session isolation, single shared `McpServer`

Gap to production: session multiplexing, auth, real bind address, headless binary, persistence model, hosted infra. Phased below.

## Topologies

| ID | Transport | Engine runs | Workbook lives | Driven by |
|----|-----------|-------------|----------------|-----------|
| **A** | stdio | User's Mac, inside the `.app` | Local file | Desktop AI (Claude Desktop, Code, Cursor) |
| **B** | Streamable HTTP, self-host | User's VPS / homelab | Server-local disk | The user's browser AI |
| **C** | Streamable HTTP, hosted | `lattice.cloud` | Multi-tenant object storage | Anyone with an account |
| **D** | postMessage / Service Worker | Browser tab (WASM) | OPFS + File System Access | In-page AI (ChatGPT Apps, etc.) |

A is shipped. B and C share an implementation (Phase 1 + 2). D is a separate build target (Phase 3).

## Phase 1 — Production-grade Streamable HTTP (v0.2.0)

Goal: the same MCP surface as stdio, reachable over HTTP, with per-client session isolation and optional auth.

### Spec alignment
- MCP `Streamable HTTP` transport (supersedes the older HTTP+SSE split)
- Single endpoint `/mcp` accepts `POST` (request) and `GET` (resumable SSE)
- `Mcp-Session-Id` response header on `initialize`; client echoes it on every subsequent request
- `DELETE /mcp` with `Mcp-Session-Id` closes a session
- `Last-Event-Id` for stream resumption after disconnect

### Crate work

**`crates/lattice-mcp/src/session.rs`** — new (~150 LOC)
```rust
pub struct SessionStore {
    sessions: DashMap<SessionId, Arc<Session>>,
    idle_ttl: Duration,           // evict after 30 min of no requests
}
pub struct Session {
    id: SessionId,
    workbook: Arc<RwLock<Workbook>>,
    created: Instant,
    last_seen: AtomicI64,
    broadcaster: broadcast::Sender<Notification>,
}
```

**`crates/lattice-mcp/src/transport/http.rs`** — extend (~+200 LOC)
- Per-request session lookup by `Mcp-Session-Id` header
- Reject requests with unknown / expired session IDs
- Per-session SSE stream (one `broadcast::Sender` per session, not global)
- Background task: evict idle sessions

**`crates/lattice-mcp/src/auth.rs`** — new (~120 LOC)
- Mode 1: `LATTICE_AUTH_TOKEN` env → bearer token check on every request
- Mode 2: `GET /.well-known/oauth-protected-resource` returns RFC 9728 metadata; defers actual OAuth to a hosted identity provider (Phase 2 wires this up — Phase 1 just exposes the stub so the discovery flow doesn't 404)
- Mode 3 (default for `127.0.0.1` bind): no auth

**`crates/lattice-server/`** — new binary crate (~100 LOC)
- Headless `lattice` for Docker / cloud deployments
- No Tauri, no WKWebView, no frontend assets bundled
- Same MCP surface; `lattice-server --bind 0.0.0.0:3141 --token-env LATTICE_AUTH_TOKEN`
- Statically links `lattice-core`, `lattice-io`, `lattice-mcp`, `lattice-charts`, `lattice-analysis`

**Integration tests** (~200 LOC, ~15 tests)
- Spin up `lattice-server` in-process, drive via `reqwest`
- Verify: initialize → session ID issued, second client gets isolated workbook, idle eviction, bearer auth accept/reject, SSE notification round-trip, DELETE closes session

### Persistence modes

Selected per-session in the `initialize` params:

1. **Ephemeral** — workbook lives in memory; lost on session close. Good for "let Claude crunch this CSV I'm pasting."
2. **Attached** — session opens a named workbook by path (self-host) or by ID (hosted). Edits persist to disk on tool-call boundaries.

Modes are advisory in Phase 1 (everything ephemeral); attached lands in Phase 2 once we have a storage layer.

### Deliverables checklist
- [ ] `crates/lattice-mcp/src/session.rs`
- [ ] `SessionStore` integrated into HTTP transport
- [ ] Bearer-token auth + bind address CLI flags
- [ ] `crates/lattice-server` headless binary
- [ ] Integration tests for session isolation, auth, eviction
- [ ] Update `PLAN.md` to mark transport Done; add HTTP setup section to README
- [ ] `make serve` target

Expected diff: ~900 LOC across 5 commits.

## Phase 2 — Hosted demo (v0.3.0)

Goal: a public URL anyone can paste into ChatGPT or claude.ai.

### Infra (v1, optimize-when-broken)
- **Docker image** — multi-stage build of `lattice-server`
- **Host** — single VM on Fly.io / Railway / Hetzner; one region; vertical scale
- **Storage** — SQLite for metadata + local disk for workbook blobs. Migrate to S3 + Postgres only if traffic warrants
- **TLS** — Caddy with auto-LE certs
- **Domain** — `lattice.cloud` or `mcp.lattice.app` (TBD)

### Account flow
1. User visits `https://lattice.cloud`, signs up (GitHub OAuth or email magic link)
2. Creates a workbook or uploads `.xlsx`
3. Copies "Add to ChatGPT/Claude" URL: `https://lattice.cloud/mcp?wb=<id>`
4. AI client OAuth-discovers via `/.well-known/oauth-protected-resource`
5. User grants access; client stores access token
6. Tool calls operate on that user's workbook
7. Web UI on `/workbook/<id>` shows the live spreadsheet for visual confirmation

### Web frontend reuse

The SolidJS frontend already talks to the Rust backend through a thin `invoke()` bridge. Swap the bridge, keep everything else:

```
frontend/src/bridge/tauri.ts   ── current
frontend/src/bridge/http.ts    ── new: same API, fetch() + WebSocket
frontend/src/bridge/index.ts   ── picks at build time via env
```

`make build-web` produces a static SPA that loads workbook data from `/api/workbook/<id>` and pushes commands to `/api/command`. The same broadcast channel that backs MCP notifications feeds the frontend over WebSocket — i.e. an AI edit shows up live in the user's web tab.

### Auth
- Account auth: GitHub OAuth (one-click) or email magic link
- MCP auth: OAuth 2.1 + PKCE, per Anthropic's "Authorization in Remote MCP Servers" spec — the Phase 1 metadata stub becomes a real authorization server now
- Workbook ACL: owner + share links (read-only or read-write, expiring)

## Phase 3 — In-browser WASM (v0.4.0+)

Goal: zero-server Lattice. The engine runs in the user's tab; the in-page AI talks to it over `postMessage`.

### Why
- **ChatGPT Apps SDK** supports MCP servers that ship a `text/html` UI. If our "server" is JavaScript-in-iframe wrapping WASM, the whole spreadsheet lives in the user's tab — no infra, no auth, no privacy concerns.
- **AI-native browsers** can host MCP servers on the page itself via `window.postMessage` or the emerging Web MCP standards.
- Folds in the "performance at 100k+ rows" pending item — once we benchmark in WASM, we'll know exactly what's slow.

### Build target
- New crate: `crates/lattice-wasm` — `wasm-bindgen` wrapper around `lattice-core` + `lattice-io` + `lattice-mcp` (no transport — uses `postMessage`)
- Exposes a single entry: `handle_request(jsonrpc_string) -> Promise<jsonrpc_string>`
- Output: `lattice.wasm` + `lattice.js` shim
- Bundle target: ≤5 MB gzipped (other browser spreadsheets ship 3–7 MB)

### File I/O in browser
- **Open** — File System Access API (`window.showOpenFilePicker`)
- **Save** — same API for export; OPFS for autosave between sessions
- `calamine` and `rust_xlsxwriter` both compile to WASM today; no new I/O work needed

### MCP transport
- Inside the iframe: a JSON-RPC handler over `window.postMessage`
- Outside (ChatGPT/Claude/browser AI): a transport that wraps `postMessage` instead of stdio/HTTP — same `Transport` trait, new impl in JS
- Apps SDK pattern: the App registers as a tool surface; the iframe runs the engine; the AI orchestrator routes tool calls into the iframe

### Risks specific to D
- Browsers that don't support in-iframe MCP yet — fall back to topology C (hosted)
- Bundle size — tree-shake unused formula functions per-deployment if needed
- Concurrent tabs — last-write-wins; CRDT story remains the same as topology C

## Topology comparison summary

| Concern | A (stdio) | B (self-host HTTP) | C (hosted) | D (WASM) |
|---------|-----------|--------------------|------------|----------|
| Install cost | `.dmg` | `docker run` | none | none |
| Per-user data isolation | OS user | session ID | account | tab |
| Offline | yes | yes (LAN) | no | yes |
| Works with claude.ai | no | yes | yes | not yet |
| Works with ChatGPT Apps | no | maybe | yes | yes (target) |
| Works with Cursor/Zed | yes | maybe | yes | no |
| Multi-user collab | no | no | yes (future) | no |
| Engine perf | native | native | native | WASM (slower) |

## Open decisions

1. **Standalone `lattice-server` binary vs `--mcp-http` flag on the Tauri binary?**
   Recommend: standalone. The Tauri binary pulls in WKWebView libs that headless deployments don't need. Keep `--mcp-http` on Tauri for "host MCP from the running GUI" — a real use case for self-host (B).

2. **SQLite-only for hosted Phase 2, or Postgres day one?**
   Recommend: SQLite. Easier ops, easy migration if needed.

3. **Per-user OAuth vs per-workbook share links?**
   Recommend: both. OAuth for "my Lattice account", share links for one-off "let Claude touch this sheet for the next hour."

4. **Apps SDK before Streamable HTTP, or after?**
   Recommend: after. HTTP works on more clients today and is the foundation Apps SDK builds on.

5. **Embed the SolidJS frontend in the hosted product, or stay headless?**
   Recommend: embed. The visual feedback ("the AI just did *that*") is the demo.

## What this leaves on the v0.2.0 list

After this plan ships, the v0.2.0 backlog in `PLAN.md` shrinks to:

- Drag and drop (files, rows, columns)
- Images in cells — insertion UX
- Chart themes / palette switching
- Trendline variants (polynomial, exponential, moving average)
- Slicer persistence to file
- Shared formula expansion on xlsx import

All independent of the browser story; each is a separate small-PR-sized task.

## References

- [MCP Streamable HTTP spec](https://modelcontextprotocol.io/specification/server/transports/)
- [Anthropic — Authorization in Remote MCP Servers](https://docs.anthropic.com/en/docs/agents-and-tools/mcp)
- [OpenAI Apps SDK](https://platform.openai.com/docs/apps)
- RFC 9728 — OAuth 2.0 Protected Resource Metadata
- `docs/PLAN.md` §4 (MCP Server Design)
- `crates/lattice-mcp/src/transport/http.rs` (existing draft)
