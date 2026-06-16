# In-Browser Lattice

> Lattice as a website: the spreadsheet runs in the browser tab, no install. Free tier is fully local (no login, no backend); a logged-in tier later adds a backend for cloud save and sharing.
>
> This supersedes the "MCP streamable HTTP transport" item deferred from v0.1.0 and reframes the earlier four-topology sketch around an actual product.

## Why

The AI-native browser moment arrived in 2025: Claude.ai, ChatGPT, Arc Max, Dia, and Comet all host MCP clients in the page. A spreadsheet that lives at a URL — rather than behind a `.dmg` — is reachable by all of them and by anyone with a browser.

The product is two tiers:

| Tier | Login | Engine | Data | Storage |
|------|-------|--------|------|---------|
| **Free** | none | WASM, in the browser tab | ≤ ~10K rows | Local file + OPFS autosave |
| **Logged-in** | account | WASM in tab, backend for sync | larger | Cloud (backend) |

The free tier is the focus of this plan (Phase 1). It needs **zero backend** — it deploys as a static site. The logged-in tier (Phase 2) adds a backend only for the things that genuinely need a server: accounts, cloud save, sharing, larger data.

## Scope: ≤10K rows, full feature set

"Smaller data" is a deliberate constraint, not a limitation to apologize for. At ~10K rows the existing engine needs **no performance work**:

- Cell storage is a sparse `HashMap` — 10K populated cells is trivial.
- The canvas grid already virtualizes — only visible rows render.
- Formula recalc is brute-force today; at 10K cells that is still sub-frame.
- WASM runs ~1.5–3× slower than native — invisible at this size.

So the deferred 100k-row / Apache-Arrow / lazy-eval work stays deferred. The free web app ships the **full feature set** — every action a desktop user can do — just with a soft row ceiling.

## Free-tier architecture (Phase 1)

```
Browser tab
├── SolidJS UI + Canvas grid          (unchanged from desktop)
├── bridge/                           (swapped: invoke() → wasm call)
│   ├── tauri.ts   — desktop build
│   ├── wasm.ts    — web build (NEW)
│   └── index.ts   — picks one at build time (NEW)
└── lattice.wasm                      (NEW — crates/lattice-wasm)
    └── lattice-core + lattice-io + lattice-charts + lattice-analysis
```

The desktop app keeps Tauri. The web app drops Tauri entirely and talks straight to a WASM module. Both share the same SolidJS frontend and the same Rust engine crates — only the bridge differs.

### Why this is a clean swap

The frontend already speaks to the backend through exactly one chokepoint: `invoke(command, args)` in `frontend/src/bridge/tauri.ts` — ~95 commands, every one a thin `invoke('name', {...})` call. The WASM port provides the **same `invoke` signature**, backed by a Rust dispatcher instead of Tauri IPC. The ~95 typed wrapper functions above it do not change.

`lattice-core`, `lattice-io`, `lattice-charts`, and `lattice-analysis` are pure Rust with no UI or OS dependencies — they compile to `wasm32-unknown-unknown` as-is. `lattice-io`'s calamine + rust_xlsxwriter both run in WASM (read/write to in-memory byte buffers).

### What changes for the browser

| Concern | Desktop (Tauri) | Web (WASM) |
|---------|-----------------|------------|
| Command dispatch | `invoke()` IPC → Rust | direct call into `lattice.wasm` |
| State | `Arc<RwLock<Workbook>>` (tokio) | `RefCell<AppState>` (single-threaded) |
| Open file | native dialog → path | File System Access API, or `<input type=file>` |
| Save file | native dialog → path | File System Access API, or Blob download |
| Autosave | disk | OPFS (Origin Private File System) |
| Menu bar | macOS native menu | in-page menu (already partly DOM) |
| `menu-event` / `workbook-changed` events | Tauri event bus | direct callbacks / no-op |

## Phase 1 — Free web app

Milestones, each a small commit or two:

1. **`crates/lattice-wasm`** — wasm-bindgen crate. One exported `invoke(command: string, argsJson: string) → resultJson`. A dispatcher mirrors the ~95 Tauri commands over a sync `AppState` (`RefCell`, no tokio). File commands take/return `Uint8Array` instead of paths.
2. **Build tooling** — `wasm-pack` build wired into the `Makefile` (`make wasm`); Vite configured to load the generated package.
3. **Frontend bridge** — `bridge/wasm.ts` implements `invoke` against the WASM module; `bridge/index.ts` selects `tauri.ts` vs `wasm.ts` from a build-time env var. The 95 wrappers in `tauri.ts` move behind `index.ts` unchanged.
4. **Browser file I/O** — open/save `.xlsx`, `.csv`, `.tsv` via File System Access API where available, upload + download fallback for Safari/Firefox.
5. **OPFS autosave** — workbook serialized to OPFS on a timer; restored on load so a refresh never loses work.
6. **Menu / events** — replace the macOS menu path with the in-page menu; stub the Tauri event listeners.
7. **Static deploy** — `make build-web` produces a static SPA; deploy to Cloudflare Pages (free, and a `cf-setup` skill exists).
8. **Verify** — cell editing, formulas, sheets, formatting, charts, file open/save all working in a browser at ~10K rows.

Phase 1 ships with no MCP — it is human-first. The in-browser MCP transport (so browser AI can drive the local app over `postMessage`) is the immediate fast-follow, Phase 1.5.

## Phase 1.5 — In-browser MCP bridge

Once the web app runs, the same MCP catalog the desktop stdio server exposes (`read_cell`, `write_cell`, `list_sheets`, `create_chart`, conditional formats, named ranges, sparklines, …) becomes drivable by any browser AI client (ChatGPT Apps SDK, Claude.ai, Arc Max, Comet, Dia) over `postMessage`. The tool catalog and protocol version (`"2024-11-05"`) come straight from `lattice-mcp` via a single WASM entry point — there is no parallel implementation in JS.

### Opt-in URL parameters

The bridge is **off by default**. A parent page must explicitly ask for it:

| Param | Effect |
|-------|--------|
| `?mcp=1` | Attach the bridge. Same-origin only. |
| `?mcpOrigin=https://embed.example,https://other.example` | Add comma-separated origins to the allowlist (same-origin is always kept). |
| `?mcpAllowAny=1` | Allow **any** origin (sets the allowlist to `*`). The visible opt-in name is deliberate — only use for testing. |
| `?mcpDebug=1` | Log every accepted / rejected message to the console. |

Without `?mcp=1`, no `message` listener is installed and the page behaves like the regular SPA.

### Origin model

Every incoming `MessageEvent` is matched against the allowlist before anything else happens:

- Default = `[window.location.origin]` (same-origin iframes only).
- `?mcpOrigin=` adds explicit origins via exact case-sensitive match.
- `?mcpAllowAny=1` opens it to `*`; the literal `"null"` opaque origin is still rejected.
- Mismatched messages are dropped silently (logged in `mcpDebug=1` mode).

### Try it: the demo page

After `make web` you get `frontend/dist/mcp-demo.html`. Serve `dist/` with any static server (`npx serve dist`, `python -m http.server`, `vite preview`) and open `/mcp-demo.html` — it embeds Lattice in an iframe with `?mcp=1` and gives you:

- Buttons for `initialize`, `tools/list`, `ping`.
- A searchable list of all tool names (came from the bridge, not hard-coded).
- A hand-edit JSON box for `tools/call`.
- A chronological message log.

### Embedding from your own page

```js
// Minimal embedder.
const iframe = document.createElement('iframe');
iframe.src = 'https://lattice.app/?mcp=1';
document.body.append(iframe);

iframe.addEventListener('load', () => {
  iframe.contentWindow.postMessage({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2024-11-05',
      capabilities: {},
      clientInfo: { name: 'my-client', version: '0.1.0' },
    },
  }, 'https://lattice.app');
});

window.addEventListener('message', (e) => {
  if (e.source !== iframe.contentWindow) return;
  if (e.data?.jsonrpc !== '2.0') return;
  console.log('lattice →', e.data);
});
```

### Cross-origin embedders

A non-same-origin parent must be added explicitly. Two equivalent ways:

```
https://lattice.app/?mcp=1&mcpOrigin=https://your-host.example
```

Or, if you control the URL and have a real reason for it, `&mcpAllowAny=1`. Prefer the explicit list.

### What's deferred to Phase 2

- **Remote MCP transport (Streamable HTTP + OAuth 2.1)** for clients that want to drive a server-hosted workbook instead of one in a tab.
- **Multiple concurrent embedders** with per-client capability scopes (the current bridge treats all callers as equally trusted).
- **A "Connected to AI assistant" inline banner** with a one-click disconnect, beyond the small dot already in the status bar.

### Code paths

- `frontend/src/mcp/server.ts` — `LatticeMcpServer`, the `postMessage` adapter.
- `frontend/src/mcp/origin.ts` — origin allowlist matching.
- `frontend/src/App.tsx` — opt-in wiring (`?mcp=1`), allowlist construction, status-bar indicator.
- `frontend/public/mcp-demo.html` — the developer demo page.
- `frontend/tests/e2e-mcp.mjs` — Chromium round-trip test (`make mcp-verify`).
- `crates/lattice-wasm/src/lib.rs` (`mcp_request`) — the WASM entry point that owns the catalog.

## Phase 2 — Logged-in backend

Only the logged-in tier needs a server. It reuses the headless-server design from the earlier plan:

- `crates/lattice-server` — headless binary (no Tauri), the engine behind an HTTP API
- Accounts (GitHub OAuth or email magic link)
- Cloud save — workbooks persisted server-side, synced across devices
- Share links — read-only or read-write, expiring
- Remote MCP — Streamable HTTP transport so browser AI drives a *hosted* workbook with OAuth 2.1
- Larger data ceilings, since the server is not bound by the tab

The WASM engine still runs in the tab for editing; the backend handles persistence and sharing. Storage starts as SQLite + disk, migrates to S3 + Postgres only if traffic demands.

## Where the desktop app fits

The `.dmg` stays for true-offline use, OS integration (Finder, native menus), and local stdio MCP (Claude Desktop, Cursor, Zed). But the web app becomes the front door — the thing a new user touches first.

## Open decisions

1. **Row ceiling — hard or soft?** Recommend soft: works past 10K, shows a gentle warning past ~25K. No hard block.
2. **Command logic — duplicate in `lattice-wasm`, or extract a shared crate?** Recommend: Phase 1 reimplements the dispatcher in `lattice-wasm` (zero risk to the shipped desktop app, self-contained). Deduplicate into a shared `lattice-app` crate later if the divergence cost shows up.
3. **Hosting** — Cloudflare Pages for the free static site. Backend host (Phase 2) TBD.
4. **Domain** — TBD (`lattice.app`, `trylattice.com`, …).

## References

- `crates/lattice-mcp/src/transport/http.rs` — existing Streamable HTTP draft (feeds Phase 2)
- `docs/PLAN.md` §4 — MCP server design
- MCP Streamable HTTP spec — https://modelcontextprotocol.io/specification/server/transports/
- File System Access API — https://developer.mozilla.org/en-US/docs/Web/API/File_System_API
- OPFS — https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system
