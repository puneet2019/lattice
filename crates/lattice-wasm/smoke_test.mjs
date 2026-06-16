// Smoke test for the compiled lattice-wasm artifact.
// Exercises the dispatcher across command categories via the Node-target build.
// Run: node crates/lattice-wasm/smoke_test.mjs
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const wasm = require('./pkg-node/lattice_wasm.js');

let pass = 0;
let fail = 0;
function check(name, cond, detail) {
  if (cond) {
    pass++;
    console.log(`  ok  ${name}`);
  } else {
    fail++;
    console.log(`FAIL  ${name}${detail ? ' — ' + detail : ''}`);
  }
}
const inv = (cmd, args) => JSON.parse(wasm.invoke(cmd, JSON.stringify(args ?? {})));

wasm.init();

// --- workbook / sheets ---
const wb = inv('new_workbook');
check('new_workbook returns sheets', Array.isArray(wb.sheets) && wb.sheets.length > 0, JSON.stringify(wb));
inv('add_sheet', { name: 'Data' });
const sheets = inv('list_sheets');
check('add_sheet + list_sheets', sheets.some((s) => s.name === 'Data'), JSON.stringify(sheets));
inv('rename_sheet', { old: 'Data', newName: 'Numbers' });
check('rename_sheet', inv('list_sheets').some((s) => s.name === 'Numbers'));

const first = wb.sheets[0];

// --- cells ---
inv('set_cell', { sheet: first, row: 0, col: 0, value: '10' });
inv('set_cell', { sheet: first, row: 1, col: 0, value: '32' });
const a1 = inv('get_cell', { sheet: first, row: 0, col: 0 });
check('set/get_cell number', a1 && a1.value === '10', JSON.stringify(a1));

// --- formula + recalc ---
inv('set_cell', { sheet: first, row: 2, col: 0, value: '=A1+A2', formula: 'A1+A2' });
const a3 = inv('get_cell', { sheet: first, row: 2, col: 0 });
check('formula =A1+A2 evaluates to 42', a3 && a3.value === '42', JSON.stringify(a3));
inv('set_cell', { sheet: first, row: 0, col: 0, value: '100' });
const a3after = inv('get_cell', { sheet: first, row: 2, col: 0 });
check('recalc after dependency change -> 132', a3after && a3after.value === '132', JSON.stringify(a3after));

// --- undo / redo ---
inv('undo');
const a1undo = inv('get_cell', { sheet: first, row: 0, col: 0 });
check('undo restores A1 to 10', a1undo && a1undo.value === '10', JSON.stringify(a1undo));
inv('redo');
const a1redo = inv('get_cell', { sheet: first, row: 0, col: 0 });
check('redo restores A1 to 100', a1redo && a1redo.value === '100', JSON.stringify(a1redo));

// --- range ---
const range = inv('get_range', { sheet: first, startRow: 0, startCol: 0, endRow: 2, endCol: 0 });
check('get_range returns 3 rows', Array.isArray(range) && range.length === 3, JSON.stringify(range));

// --- formatting ---
inv('format_cells', { sheet: first, startRow: 0, startCol: 0, endRow: 0, endCol: 0, format: { bold: true } });
const bold = inv('get_cell', { sheet: first, row: 0, col: 0 });
check('format_cells bold', bold && bold.bold === true, JSON.stringify(bold));

// --- error handling ---
let threw = false;
try {
  wasm.invoke('not_a_real_command', '{}');
} catch (e) {
  threw = true;
}
check('unknown command throws', threw);

// --- xlsx round-trip ---
const bytes = wasm.save_xlsx();
check('save_xlsx produces bytes', bytes instanceof Uint8Array && bytes.length > 0, `len=${bytes && bytes.length}`);
const reopened = JSON.parse(wasm.open_xlsx(bytes));
check('open_xlsx round-trip returns sheets', Array.isArray(reopened.sheets) && reopened.sheets.length > 0, JSON.stringify(reopened));
const a1rt = inv('get_cell', { sheet: reopened.sheets[0], row: 0, col: 0 });
check('xlsx round-trip preserves A1=100', a1rt && a1rt.value === '100', JSON.stringify(a1rt));

// --- csv export ---
const csv = wasm.export_csv_bytes(reopened.sheets[0]);
check('export_csv_bytes produces bytes', csv instanceof Uint8Array && csv.length > 0, `len=${csv && csv.length}`);

// --- MCP (the Phase 1.5 surface) ---
const mcp = (msg) => {
  const r = wasm.mcp_request(JSON.stringify(msg));
  return r === undefined ? undefined : JSON.parse(r);
};
const initResp = mcp({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} });
check(
  'mcp initialize returns serverInfo',
  initResp?.result?.serverInfo?.name === 'lattice' && initResp.result.protocolVersion,
  JSON.stringify(initResp),
);
mcp({ jsonrpc: '2.0', method: 'initialized' }); // notification (no response)
const toolsList = mcp({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
const toolCount = toolsList?.result?.tools?.length ?? 0;
check('mcp tools/list returns tools', toolCount > 30, `tools=${toolCount}`);
check(
  'mcp tools include read_cell + write_cell',
  toolsList.result.tools.some((t) => t.name === 'read_cell')
    && toolsList.result.tools.some((t) => t.name === 'write_cell'),
);
// Round-trip through MCP: write_cell then read_cell.
const sheetName = reopened.sheets[0];
mcp({
  jsonrpc: '2.0', id: 3, method: 'tools/call',
  params: { name: 'write_cell', arguments: { sheet: sheetName, cell_ref: 'C1', value: 777 } },
});
const readResp = mcp({
  jsonrpc: '2.0', id: 4, method: 'tools/call',
  params: { name: 'read_cell', arguments: { sheet: sheetName, cell_ref: 'C1' } },
});
// tools/call wraps the result in { content: [{type:'text', text: JSON.stringify(...)}] }
// (MCP spec). Some servers return the raw value too. Accept either shape.
const readPayload = readResp?.result?.content?.[0]?.text
  ? JSON.parse(readResp.result.content[0].text)
  : readResp?.result;
check(
  'mcp tools/call write+read_cell round-trip',
  readPayload?.value === 777 || readPayload?.value === '777',
  JSON.stringify(readResp),
);
const notifResp = mcp({ jsonrpc: '2.0', method: 'tools/list' }); // notification
check('mcp notification returns no response', notifResp === undefined, JSON.stringify(notifResp));

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
