# Google Sheets Feature Parity Reference

> Comprehensive audit of every Google Sheets feature vs Lattice.
> See full audit in session transcript for 582-item breakdown.

**Parity: 203/582 (35%) implemented, 48 partial, 321 missing**

## P0 Blockers — Core tasks impossible without these

1. **Borders** — zero border functionality (Format Cells tab is a stub)
2. **Conditional formatting UI** — backend exists but no GUI to create/manage rules
3. **Data validation / Dropdown cells** — backend exists but no GUI
4. **Filter / Auto-filter UI** — no way to filter data via GUI
5. **Hide/unhide rows/columns UI** — backend exists but no GUI

## P1 High Impact

6. Page Up / Page Down navigation
7. Fn+F4 absolute/relative reference toggle
8. Formula argument hints tooltip after `(`
9. Point-and-click range selection in formulas
10. Clear formatting (Cmd+\)
11. Min/Max in status bar
12. Auto-save indicator
13. Print (Cmd+P)
14. Sort UI (Data menu)
15. Named ranges UI

## Major Missing Categories

| Category | Implemented | Missing |
|----------|------------|---------|
| Borders | 0/12 | All |
| Conditional Formatting UI | 0/17 | All |
| Data Validation UI | 0/10 | All |
| Filter UI | 0/8 | All |
| Print | 0/11 | All |
| Menu Bar (View/Insert/Format/Data) | 5/50 | 45 |
| Context Menu items | 8/38 | 30 |
| Missing Keyboard Shortcuts | 44/80 | 36 |

## Where Lattice Wins

| Feature | Google Sheets | Lattice |
|---------|-------------|---------|
| MCP/AI integration | None | 40+ tools, stdio + HTTP |
| Native performance | Browser-limited | Rust, <15MB |
| Offline | Requires browser | Fully offline |
| Dark mode | Limited | Full system theme |
| Split panes | Limited | 4-quadrant |

## Sources
- https://support.google.com/docs/answer/181110 (keyboard shortcuts)
- https://support.google.com/docs/answer/78413 (conditional formatting)
- https://support.google.com/docs/answer/190718 (chart types)
