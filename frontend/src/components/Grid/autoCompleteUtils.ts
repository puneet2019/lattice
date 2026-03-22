/**
 * Pure utility functions for auto-complete suggestions.
 * Kept separate from the component to allow unit testing without DOM.
 */

/**
 * Collect unique non-empty string values for a given column from the cell cache.
 * Excludes numeric values and data URL images.
 */
export function getColumnSuggestions(
  cellCache: Map<string, { value: string }>,
  col: number,
): string[] {
  const seen = new Set<string>();
  cellCache.forEach((cell, key) => {
    const parts = key.split(':');
    if (parseInt(parts[1], 10) !== col) return;
    const v = cell.value?.trim();
    if (!v) return;
    // Skip numeric values, formulas, and data URLs
    if (!isNaN(Number(v))) return;
    if (v.startsWith('data:image/')) return;
    seen.add(v);
  });
  return Array.from(seen).sort();
}

/**
 * Filter suggestions by prefix match (case-insensitive), excluding exact match.
 */
export function filterSuggestions(
  suggestions: string[],
  input: string,
): string[] {
  const lower = input.toLowerCase().trim();
  if (!lower) return [];
  return suggestions.filter((s) => {
    const sl = s.toLowerCase();
    return sl.startsWith(lower) && sl !== lower;
  });
}
