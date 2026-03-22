import type { Component } from 'solid-js';
import { createSignal, Show } from 'solid-js';
import { findInSheet, setCell } from '../bridge/tauri';
import type { FindResult } from '../bridge/tauri';

export interface FindBarProps {
  /** The active sheet name for searching. */
  activeSheet: string;
  /** Whether to show replace controls (Cmd+H mode). */
  showReplace: boolean;
  /** Called when the user closes the find bar. */
  onClose: () => void;
  /** Called to navigate the grid to a found cell. */
  onNavigateToCell: (row: number, col: number) => void;
  /** Called to update the status bar message. */
  onStatusChange: (message: string) => void;
  /** Called after a replace writes data (to trigger grid refresh). */
  onDataChanged: () => void;
}

const FindBar: Component<FindBarProps> = (props) => {
  const [query, setQuery] = createSignal('');
  const [replaceText, setReplaceText] = createSignal('');
  const [caseSensitive, setCaseSensitive] = createSignal(false);
  const [useRegex, setUseRegex] = createSignal(false);
  const [results, setResults] = createSignal<FindResult[]>([]);
  const [currentIndex, setCurrentIndex] = createSignal(-1);

  let searchInputRef: HTMLInputElement | undefined;

  /** Filter results based on case-sensitivity and regex options. */
  function filterResults(rawResults: FindResult[], q: string): FindResult[] {
    if (!q) return [];
    if (useRegex()) {
      try {
        const flags = caseSensitive() ? '' : 'i';
        const re = new RegExp(q, flags);
        return rawResults.filter((r) => re.test(r.value));
      } catch {
        // Invalid regex -- return no results
        return [];
      }
    }
    if (caseSensitive()) {
      return rawResults.filter((r) => r.value.includes(q));
    }
    const lowerQ = q.toLowerCase();
    return rawResults.filter((r) => r.value.toLowerCase().includes(lowerQ));
  }

  async function doSearch() {
    const q = query();
    if (!q) {
      setResults([]);
      setCurrentIndex(-1);
      props.onStatusChange('');
      return;
    }

    try {
      const rawResults = await findInSheet(props.activeSheet, q);
      const filtered = filterResults(rawResults, q);
      setResults(filtered);
      if (filtered.length > 0) {
        setCurrentIndex(0);
        props.onNavigateToCell(filtered[0].row, filtered[0].col);
        props.onStatusChange(`${filtered.length} match${filtered.length === 1 ? '' : 'es'} found`);
      } else {
        setCurrentIndex(-1);
        props.onStatusChange('No matches found');
      }
    } catch {
      // Tauri not available or search error
      setResults([]);
      setCurrentIndex(-1);
      props.onStatusChange('Search failed');
    }
  }

  function goToNext() {
    const r = results();
    if (r.length === 0) return;
    const next = (currentIndex() + 1) % r.length;
    setCurrentIndex(next);
    props.onNavigateToCell(r[next].row, r[next].col);
    props.onStatusChange(`Match ${next + 1} of ${r.length}`);
  }

  function goToPrevious() {
    const r = results();
    if (r.length === 0) return;
    const prev = (currentIndex() - 1 + r.length) % r.length;
    setCurrentIndex(prev);
    props.onNavigateToCell(r[prev].row, r[prev].col);
    props.onStatusChange(`Match ${prev + 1} of ${r.length}`);
  }

  async function handleReplace() {
    const r = results();
    const idx = currentIndex();
    if (idx < 0 || idx >= r.length) return;

    const match = r[idx];
    const replacement = replaceText();
    try {
      await setCell(props.activeSheet, match.row, match.col, replacement, undefined);
      props.onDataChanged();
      props.onStatusChange(`Replaced at ${match.row + 1}:${match.col + 1}`);
      // Re-search to refresh results
      await doSearch();
    } catch {
      props.onStatusChange('Replace failed');
    }
  }

  async function handleReplaceAll() {
    const r = results();
    if (r.length === 0) return;

    const replacement = replaceText();
    const promises = r.map((match) =>
      setCell(props.activeSheet, match.row, match.col, replacement, undefined).catch(() => {}),
    );
    await Promise.all(promises);
    props.onDataChanged();
    props.onStatusChange(`Replaced ${r.length} match${r.length === 1 ? '' : 'es'}`);
    // Re-search to refresh results
    await doSearch();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        goToPrevious();
      } else {
        if (results().length === 0) {
          doSearch();
        } else {
          goToNext();
        }
      }
    }
  }

  function handleSearchInput(value: string) {
    setQuery(value);
    doSearch();
  }

  // Auto-focus search input on mount
  requestAnimationFrame(() => {
    searchInputRef?.focus();
  });

  const matchInfo = () => {
    const r = results();
    const idx = currentIndex();
    if (r.length === 0 && query()) return 'No results';
    if (r.length === 0) return '';
    return `${idx + 1} of ${r.length}`;
  };

  return (
    <div class="find-bar" onKeyDown={handleKeyDown}>
      <div class="find-bar-row">
        <input
          ref={searchInputRef}
          class="find-bar-input"
          type="text"
          placeholder="Find..."
          value={query()}
          onInput={(e) => handleSearchInput(e.currentTarget.value)}
        />
        <span class="find-bar-match-info">{matchInfo()}</span>
        <button
          class={`find-bar-toggle ${caseSensitive() ? 'active' : ''}`}
          title="Case sensitive"
          onClick={() => { setCaseSensitive(!caseSensitive()); doSearch(); }}
        >
          Aa
        </button>
        <button
          class={`find-bar-toggle ${useRegex() ? 'active' : ''}`}
          title="Regular expression"
          onClick={() => { setUseRegex(!useRegex()); doSearch(); }}
        >
          .*
        </button>
        <button class="find-bar-btn" title="Previous (Shift+Enter)" onClick={goToPrevious}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M2 8L6 4l4 4" />
          </svg>
        </button>
        <button class="find-bar-btn" title="Next (Enter)" onClick={goToNext}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M2 4l4 4 4-4" />
          </svg>
        </button>
        <button class="find-bar-close" title="Close (Esc)" onClick={props.onClose}>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M2 2l8 8M10 2l-8 8" />
          </svg>
        </button>
      </div>
      <Show when={props.showReplace}>
        <div class="find-bar-row">
          <input
            class="find-bar-input"
            type="text"
            placeholder="Replace with..."
            value={replaceText()}
            onInput={(e) => setReplaceText(e.currentTarget.value)}
          />
          <button class="find-bar-btn-text" onClick={handleReplace}>
            Replace
          </button>
          <button class="find-bar-btn-text" onClick={handleReplaceAll}>
            Replace All
          </button>
        </div>
      </Show>
    </div>
  );
};

export default FindBar;
