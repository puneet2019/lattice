/**
 * Origin validation for the in-browser MCP bridge.
 *
 * Every `message` event the bridge receives carries the embedder's origin in
 * `event.origin`. We refuse to dispatch any MCP request whose origin is not
 * on the allowlist — otherwise an unrelated page in another iframe / tab
 * could drive the user's workbook through a stray `postMessage`.
 *
 * The allowlist is either an explicit array of origins (most secure) or the
 * sentinel `"*"` (any origin, must be explicitly opted in by the embedder).
 * The default at the call site is **same-origin only**.
 */

/** An allowlist value: a list of exact origins, or the sentinel `"*"`. */
export type AllowedOrigins = readonly string[] | '*';

/**
 * Returns `true` if `origin` is permitted by `allowed`.
 *
 * Rules:
 *  - `"*"` allows any non-empty origin (the `"null"` opaque origin is still
 *    rejected — it shows up for sandboxed iframes / `file://` and is rarely
 *    what an embedder actually wants to grant).
 *  - An array allows an exact case-sensitive match against any entry.
 *  - Empty / missing origin is always rejected.
 *  - An empty array always rejects (i.e. "no one allowed").
 */
export function isOriginAllowed(
  origin: string | undefined | null,
  allowed: AllowedOrigins,
): boolean {
  if (!origin) return false;
  // `null` (string) is the literal value browsers put on the wire for opaque
  // origins. Never treat it as a real origin — even under `"*"`.
  if (origin === 'null') return false;
  if (allowed === '*') return true;
  for (const entry of allowed) {
    if (entry === origin) return true;
  }
  return false;
}

/**
 * Parse a comma-separated origin list from a URL search parameter.
 *
 * Trims whitespace and drops empty entries. Returns an empty array if the
 * input is null/empty — the caller decides what an empty list means.
 */
export function parseOriginList(raw: string | null | undefined): string[] {
  if (!raw) return [];
  return raw
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}
