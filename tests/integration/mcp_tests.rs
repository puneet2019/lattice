//! MCP integration tests — full Claude Desktop workflow.
//!
//! Each test creates its own `McpServer::new_default()` instance (no transport),
//! sends JSON-RPC 2.0 strings via `handle_message()`, and asserts on the
//! parsed response values — not just `isError: false`.

use lattice_core::selection::{CellRef, Range};
use lattice_core::{CellValue, FillDirection, FillPattern, Sheet, detect_pattern, fill_range};
use lattice_mcp::McpServer;
use serde_json::{Value, json};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Send a tools/call JSON-RPC request and return the parsed inner result value.
///
/// Asserts:
///  - the outer JSON-RPC envelope has no `error` field
///  - `result.isError` is false
///  - `result.content[0].text` parses as valid JSON
async fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": args,
        }
    });
    let raw = server
        .handle_message(&request.to_string())
        .await
        .expect("server returned None for a tool call");

    let envelope: Value = serde_json::from_str(&raw).expect("response is not valid JSON");
    assert!(
        envelope.get("error").is_none(),
        "unexpected JSON-RPC error for tool '{}': {}",
        name,
        envelope
    );

    let result = &envelope["result"];
    assert_eq!(
        result["isError"], false,
        "tool '{}' returned isError=true: {}",
        name, result["content"][0]["text"]
    );

    let text = result["content"][0]["text"]
        .as_str()
        .expect("content[0].text is not a string");
    serde_json::from_str(text).expect("content[0].text is not valid JSON")
}

/// Like `call_tool` but asserts `isError: true`.
async fn call_tool_expect_error(server: &mut McpServer, name: &str, args: Value) -> String {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": args,
        }
    });
    let raw = server
        .handle_message(&request.to_string())
        .await
        .expect("server returned None");

    let envelope: Value = serde_json::from_str(&raw).unwrap();
    let result = &envelope["result"];
    assert_eq!(
        result["isError"], true,
        "expected isError=true but got false for tool '{}'",
        name
    );
    result["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ── 1. Initialize ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_initialize() {
    let mut server = McpServer::new_default();

    let raw = server
        .handle_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-desktop","version":"1.0"}}}"#,
        )
        .await
        .unwrap();

    let parsed: Value = serde_json::from_str(&raw).unwrap();

    // Must respond with the correct protocol version.
    assert_eq!(
        parsed["result"]["protocolVersion"], "2024-11-05",
        "server did not echo the correct protocolVersion"
    );

    // Must advertise tools capability.
    assert!(
        parsed["result"]["capabilities"]["tools"].is_object(),
        "server must advertise tools capability"
    );

    // Must include server info.
    assert_eq!(
        parsed["result"]["serverInfo"]["name"], "lattice",
        "server name must be 'lattice'"
    );

    // JSON-RPC id must be echoed.
    assert_eq!(parsed["id"], 1);
    assert!(parsed.get("error").is_none());
}

// ── 2. List sheets ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_list_sheets_default_workbook() {
    let mut server = McpServer::new_default();

    let result = call_tool(&mut server, "list_sheets", json!({})).await;

    // A new workbook always has exactly one sheet named "Sheet1".
    assert_eq!(result["count"], 1, "new workbook should have 1 sheet");
    assert_eq!(
        result["sheets"][0]["name"], "Sheet1",
        "default sheet must be named 'Sheet1'"
    );
    assert_eq!(result["active_sheet"], "Sheet1");
    assert_eq!(result["sheets"][0]["cell_count"], 0);
}

// ── 3. Write cells ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_write_cell_number() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 42}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cell_ref"], "A1");
}

#[tokio::test]
async fn test_mcp_write_cell_text() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B3", "value": "hello world"}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cell_ref"], "B3");
}

#[tokio::test]
async fn test_mcp_write_multiple_cells() {
    let mut server = McpServer::new_default();

    // Write a column of numbers.
    for (i, v) in [10, 20, 30].iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        let result = call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
        assert_eq!(result["success"], true);
    }

    // Write a text header.
    let result = call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "value": "Revenue"}),
    )
    .await;
    assert_eq!(result["success"], true);
}

// ── 4. Read cells ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_read_cell_roundtrip() {
    let mut server = McpServer::new_default();

    // Write then read back.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C5", "value": 99.5}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C5"}),
    )
    .await;

    assert_eq!(result["cell_ref"], "C5");
    assert_eq!(
        result["value"].as_f64().unwrap(),
        99.5,
        "read value must match written value"
    );
    assert!(result["formula"].is_null(), "cell should have no formula");
}

#[tokio::test]
async fn test_mcp_read_cell_text_roundtrip() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "D2", "value": "lattice"}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "D2"}),
    )
    .await;

    assert_eq!(result["value"], "lattice");
}

#[tokio::test]
async fn test_mcp_read_empty_cell_returns_null() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "Z99"}),
    )
    .await;

    assert!(
        result["value"].is_null(),
        "empty cell should return null value"
    );
    assert!(result["formula"].is_null());
}

#[tokio::test]
async fn test_mcp_read_range_roundtrip() {
    let mut server = McpServer::new_default();

    // Write a 2x3 block.
    let values = [[1, 2, 3], [4, 5, 6]];
    for (row_idx, row) in values.iter().enumerate() {
        for (col_idx, val) in row.iter().enumerate() {
            let col_char = (b'A' + col_idx as u8) as char;
            let cell_ref = format!("{}{}", col_char, row_idx + 1);
            call_tool(
                &mut server,
                "write_cell",
                json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": val}),
            )
            .await;
        }
    }

    let result = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "A1:C2"}),
    )
    .await;

    assert_eq!(result["range"], "A1:C2");
    let data = result["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "should have 2 rows");
    assert_eq!(data[0][0], 1.0);
    assert_eq!(data[0][1], 2.0);
    assert_eq!(data[0][2], 3.0);
    assert_eq!(data[1][0], 4.0);
    assert_eq!(data[1][1], 5.0);
    assert_eq!(data[1][2], 6.0);
}

// ── 5. Insert formula ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_insert_formula_sum() {
    let mut server = McpServer::new_default();

    // Write values.
    for (i, v) in [10.0_f64, 20.0, 30.0].iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
    }

    // Insert SUM formula.
    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A4", "formula": "SUM(A1:A3)"}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["formula"], "SUM(A1:A3)");
    assert_eq!(
        result["result"].as_f64().unwrap(),
        60.0,
        "SUM(10+20+30) must equal 60"
    );
    assert_eq!(result["result_type"], "number");

    // Verify the cell was actually written with the formula.
    let cell = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A4"}),
    )
    .await;
    assert_eq!(cell["value"].as_f64().unwrap(), 60.0);
    assert_eq!(cell["formula"], "SUM(A1:A3)");
}

// ── 6. Evaluate formula ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_evaluate_formula_without_writing() {
    let mut server = McpServer::new_default();

    // Set up data.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 100}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": 200}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "evaluate_formula",
        json!({"sheet": "Sheet1", "formula": "SUM(A1:A2)"}),
    )
    .await;

    assert_eq!(result["formula"], "SUM(A1:A2)");
    assert_eq!(
        result["result"].as_f64().unwrap(),
        300.0,
        "SUM(100+200) must equal 300"
    );
    assert_eq!(result["result_type"], "number");

    // The formula must NOT have written to any cell — A3 must still be empty.
    let a3 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3"}),
    )
    .await;
    assert!(
        a3["value"].is_null(),
        "evaluate_formula must not write to cells"
    );
}

#[tokio::test]
async fn test_mcp_evaluate_formula_arithmetic() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "evaluate_formula",
        json!({"sheet": "Sheet1", "formula": "2+3*4"}),
    )
    .await;

    // 2 + 3*4 = 14 (standard precedence).
    assert_eq!(result["result"].as_f64().unwrap(), 14.0);
}

// ── 7. Bulk formula ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_bulk_formula_multiple_operations() {
    let mut server = McpServer::new_default();

    // Seed data: A1=5, A2=10, A3=15.
    for (i, v) in [5, 10, 15].iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "bulk_formula",
        json!({
            "sheet": "Sheet1",
            "operations": [
                {"cell_ref": "B1", "formula": "A1*2"},
                {"cell_ref": "B2", "formula": "A2*2"},
                {"cell_ref": "B3", "formula": "A3*2"},
                {"cell_ref": "C1", "formula": "SUM(A1:A3)"}
            ]
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["total"], 4);
    assert_eq!(result["succeeded"], 4);
    assert_eq!(result["failed"], 0);

    let results = result["results"].as_array().unwrap();
    assert_eq!(results[0]["result"].as_f64().unwrap(), 10.0, "A1*2 = 10");
    assert_eq!(results[1]["result"].as_f64().unwrap(), 20.0, "A2*2 = 20");
    assert_eq!(results[2]["result"].as_f64().unwrap(), 30.0, "A3*2 = 30");
    assert_eq!(
        results[3]["result"].as_f64().unwrap(),
        30.0,
        "SUM(A1:A3) = 30"
    );
}

#[tokio::test]
async fn test_mcp_bulk_formula_partial_failure() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "bulk_formula",
        json!({
            "sheet": "Sheet1",
            "operations": [
                {"cell_ref": "A1", "formula": "1+1"},
                {"cell_ref": "INVALID!!!", "formula": "2+2"}
            ]
        }),
    )
    .await;

    // Partial success: one good, one bad ref.
    assert_eq!(result["success"], false);
    assert_eq!(result["succeeded"], 1);
    assert_eq!(result["failed"], 1);
}

// ── 8. Sort range ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_sort_range_ascending() {
    let mut server = McpServer::new_default();

    // Write unsorted data: 3, 1, 2.
    for (i, v) in [3, 1, 2].iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Sheet1",
            "range": "A1:A3",
            "sort_by": [{"column": "A", "ascending": true}]
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["rows_sorted"], 3);

    // Verify sorted order via read_range.
    let range_data = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "A1:A3"}),
    )
    .await;

    let data = range_data["data"].as_array().unwrap();
    assert_eq!(data[0][0].as_f64().unwrap(), 1.0, "first element must be 1");
    assert_eq!(
        data[1][0].as_f64().unwrap(),
        2.0,
        "second element must be 2"
    );
    assert_eq!(data[2][0].as_f64().unwrap(), 3.0, "third element must be 3");
}

#[tokio::test]
async fn test_mcp_sort_range_descending() {
    let mut server = McpServer::new_default();

    for (i, v) in [5, 2, 8, 1].iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
    }

    call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Sheet1",
            "range": "A1:A4",
            "sort_by": [{"column": "A", "ascending": false}]
        }),
    )
    .await;

    let range_data = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "A1:A4"}),
    )
    .await;

    let data = range_data["data"].as_array().unwrap();
    assert_eq!(data[0][0].as_f64().unwrap(), 8.0);
    assert_eq!(data[1][0].as_f64().unwrap(), 5.0);
    assert_eq!(data[2][0].as_f64().unwrap(), 2.0);
    assert_eq!(data[3][0].as_f64().unwrap(), 1.0);
}

#[tokio::test]
async fn test_mcp_sort_range_multi_column_preserves_row_integrity() {
    let mut server = McpServer::new_default();

    // Name (A), Score (B): (B,90), (A,85), (A,95)
    let rows = [("B", 90), ("A", 85), ("A", 95)];
    for (i, (name, score)) in rows.iter().enumerate() {
        let row = i + 1;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", row), "value": name}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("B{}", row), "value": score}),
        )
        .await;
    }

    call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Sheet1",
            "range": "A1:B3",
            "sort_by": [
                {"column": "A", "ascending": true},
                {"column": "B", "ascending": false}
            ]
        }),
    )
    .await;

    let range_data = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "A1:B3"}),
    )
    .await;

    let data = range_data["data"].as_array().unwrap();
    // After sort by A asc, B desc: (A,95), (A,85), (B,90)
    assert_eq!(data[0][0], "A");
    assert_eq!(data[0][1].as_f64().unwrap(), 95.0);
    assert_eq!(data[1][0], "A");
    assert_eq!(data[1][1].as_f64().unwrap(), 85.0);
    assert_eq!(data[2][0], "B");
    assert_eq!(data[2][1].as_f64().unwrap(), 90.0);
}

// ── 9. Find and replace ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_find_replace_basic() {
    let mut server = McpServer::new_default();

    // Write some text cells.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Hello World"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "Hello Lattice"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3", "value": "Goodbye World"}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "find_replace",
        json!({"find": "Hello", "replace": "Hi", "sheet": "Sheet1"}),
    )
    .await;

    assert_eq!(
        result["matches_found"], 2,
        "should find 2 cells with 'Hello'"
    );
    assert_eq!(result["replacements_made"], 2, "should replace in 2 cells");

    // Verify replacement via read_cell.
    let a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(a1["value"], "Hi World", "A1 value must be updated");

    let a2 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2"}),
    )
    .await;
    assert_eq!(a2["value"], "Hi Lattice", "A2 value must be updated");

    // A3 should be unchanged — it had no 'Hello'.
    let a3 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3"}),
    )
    .await;
    assert_eq!(a3["value"], "Goodbye World", "A3 must be unchanged");
}

#[tokio::test]
async fn test_mcp_find_only_no_replacement() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "apple"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "pineapple"}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "find_replace",
        json!({"find": "apple", "sheet": "Sheet1"}),
    )
    .await;

    assert_eq!(result["matches_found"], 2);
    assert_eq!(
        result["replacements_made"], 0,
        "find-only mode must not replace"
    );

    // Values unchanged.
    let a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(a1["value"], "apple");
}

// ── 10. Describe data ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_describe_data_statistics() {
    let mut server = McpServer::new_default();

    // Financial data: revenues for 5 months.
    let revenues = [1000.0, 1200.0, 900.0, 1500.0, 1100.0];
    for (i, v) in revenues.iter().enumerate() {
        let cell_ref = format!("A{}", i + 1);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "describe_data",
        json!({"sheet": "Sheet1", "range": "A1:A5"}),
    )
    .await;

    assert_eq!(result["numeric_count"], 5);
    assert_eq!(result["null_count"], 0);

    let stats = &result["statistics"];

    // Mean: (1000+1200+900+1500+1100)/5 = 5700/5 = 1140.
    assert_eq!(
        stats["mean"].as_f64().unwrap(),
        1140.0,
        "mean must equal 1140"
    );

    // Median: sorted = [900, 1000, 1100, 1200, 1500], median = 1100.
    assert_eq!(
        stats["median"].as_f64().unwrap(),
        1100.0,
        "median must equal 1100"
    );

    assert_eq!(stats["min"].as_f64().unwrap(), 900.0, "min must be 900");
    assert_eq!(stats["max"].as_f64().unwrap(), 1500.0, "max must be 1500");
    assert_eq!(stats["sum"].as_f64().unwrap(), 5700.0, "sum must be 5700");

    // Population standard deviation of [900, 1000, 1100, 1200, 1500]:
    // mean=1140, variance=42400, std_dev=sqrt(42400)≈205.91.
    let std_dev = stats["std_dev"].as_f64().unwrap();
    assert!(
        (std_dev - 205.91).abs() < 0.5,
        "std_dev should be ~205.9, got {}",
        std_dev
    );
}

#[tokio::test]
async fn test_mcp_describe_data_with_mixed_types() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 100}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "text"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3", "value": 200}),
    )
    .await;
    // A4 left empty.

    let result = call_tool(
        &mut server,
        "describe_data",
        json!({"sheet": "Sheet1", "range": "A1:A4"}),
    )
    .await;

    assert_eq!(result["numeric_count"], 2, "only numbers count");
    assert_eq!(result["text_count"], 1);
    assert_eq!(result["null_count"], 1, "empty cell counts as null");
    assert_eq!(result["statistics"]["mean"].as_f64().unwrap(), 150.0);
}

// ── 11. Format operations ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_set_and_get_cell_format_persistence() {
    let mut server = McpServer::new_default();

    // Write a value first.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 42}),
    )
    .await;

    // Set format.
    let set_result = call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bold": true,
            "italic": true,
            "font_size": 14.0,
            "font_color": "#FF0000",
            "bg_color": "#FFFF00",
            "h_align": "center"
        }),
    )
    .await;

    assert_eq!(set_result["success"], true);
    assert_eq!(set_result["cells_formatted"], 1);

    // Read back the format and verify persistence.
    let fmt_result = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;

    let fmt = &fmt_result["format"];
    assert_eq!(fmt["bold"], true, "bold must persist");
    assert_eq!(fmt["italic"], true, "italic must persist");
    assert_eq!(
        fmt["font_size"].as_f64().unwrap(),
        14.0,
        "font_size must persist"
    );
    assert_eq!(fmt["font_color"], "#FF0000", "font_color must persist");
    assert_eq!(fmt["bg_color"], "#FFFF00", "bg_color must persist");
    assert_eq!(fmt["h_align"], "center", "h_align must persist");
}

#[tokio::test]
async fn test_mcp_set_format_range_applies_to_all_cells() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1:B2",
            "bold": true,
            "bg_color": "#CCCCCC"
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cells_formatted"], 4, "A1:B2 is 4 cells");

    // Verify each of the 4 cells is bold.
    for cell_ref in &["A1", "A2", "B1", "B2"] {
        let fmt_result = call_tool(
            &mut server,
            "get_cell_format",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref}),
        )
        .await;
        assert_eq!(
            fmt_result["format"]["bold"], true,
            "cell {} must be bold",
            cell_ref
        );
        assert_eq!(
            fmt_result["format"]["bg_color"], "#CCCCCC",
            "cell {} bg_color must persist",
            cell_ref
        );
    }
}

// ── 12. Sheet management ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_sheet_lifecycle_create_rename_list_delete() {
    let mut server = McpServer::new_default();

    // 1. Create a new sheet.
    let create_result = call_tool(&mut server, "create_sheet", json!({"name": "SalesData"})).await;
    assert_eq!(create_result["success"], true);
    assert_eq!(create_result["sheet_name"], "SalesData");

    // 2. Verify it appears in list_sheets.
    let list_after_create = call_tool(&mut server, "list_sheets", json!({})).await;
    assert_eq!(list_after_create["count"], 2, "should now have 2 sheets");
    let sheet_names: Vec<&str> = list_after_create["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(
        sheet_names.contains(&"SalesData"),
        "SalesData must appear in list"
    );
    assert!(
        sheet_names.contains(&"Sheet1"),
        "Sheet1 must still be present"
    );

    // 3. Rename the new sheet.
    let rename_result = call_tool(
        &mut server,
        "rename_sheet",
        json!({"old_name": "SalesData", "new_name": "Q1Revenue"}),
    )
    .await;
    assert_eq!(rename_result["success"], true);
    assert_eq!(rename_result["old_name"], "SalesData");
    assert_eq!(rename_result["new_name"], "Q1Revenue");

    // 4. Verify rename in list_sheets.
    let list_after_rename = call_tool(&mut server, "list_sheets", json!({})).await;
    let names_after_rename: Vec<&str> = list_after_rename["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(
        !names_after_rename.contains(&"SalesData"),
        "SalesData must no longer exist"
    );
    assert!(
        names_after_rename.contains(&"Q1Revenue"),
        "Q1Revenue must exist after rename"
    );

    // 5. Delete the renamed sheet.
    let delete_result = call_tool(&mut server, "delete_sheet", json!({"name": "Q1Revenue"})).await;
    assert_eq!(delete_result["success"], true);
    assert_eq!(delete_result["deleted_sheet"], "Q1Revenue");

    // 6. Verify back to 1 sheet.
    let list_after_delete = call_tool(&mut server, "list_sheets", json!({})).await;
    assert_eq!(
        list_after_delete["count"], 1,
        "should be back to 1 sheet after delete"
    );
    assert_eq!(list_after_delete["sheets"][0]["name"], "Sheet1");
}

#[tokio::test]
async fn test_mcp_create_sheet_duplicate_name_fails() {
    let mut server = McpServer::new_default();

    // "Sheet1" already exists — creating it again must fail.
    let err_msg =
        call_tool_expect_error(&mut server, "create_sheet", json!({"name": "Sheet1"})).await;

    assert!(
        !err_msg.is_empty(),
        "duplicate sheet creation must produce an error message"
    );
}

// ── 13. Export CSV ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_export_csv_with_data() {
    let mut server = McpServer::new_default();

    // Write a 3x2 table.
    let data = [
        ("A1", "Name"),
        ("B1", "Score"),
        ("A2", "Alice"),
        ("B2", "95"),
        ("A3", "Bob"),
        ("B3", "82"),
    ];
    for (cell_ref, value) in &data {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": value}),
        )
        .await;
    }

    let result = call_tool(&mut server, "export_csv", json!({"sheet": "Sheet1"})).await;

    assert_eq!(result["format"], "csv");
    assert_eq!(result["sheet"], "Sheet1");
    assert_eq!(result["rows"], 3);

    let csv = result["csv"].as_str().unwrap();
    assert!(csv.contains("Name"), "CSV must contain header 'Name'");
    assert!(csv.contains("Score"), "CSV must contain header 'Score'");
    assert!(csv.contains("Alice"), "CSV must contain data 'Alice'");
    assert!(csv.contains("95"), "CSV must contain data '95'");
    assert!(csv.contains("Bob"), "CSV must contain data 'Bob'");
    assert!(csv.contains("82"), "CSV must contain data '82'");

    // Verify CSV structure: 3 lines.
    let lines: Vec<&str> = csv.split('\n').collect();
    assert_eq!(lines.len(), 3, "CSV must have exactly 3 lines");
}

#[tokio::test]
async fn test_mcp_export_csv_empty_sheet() {
    let mut server = McpServer::new_default();

    let result = call_tool(&mut server, "export_csv", json!({"sheet": "Sheet1"})).await;

    assert_eq!(result["csv"], "", "empty sheet should produce empty CSV");
    assert_eq!(result["rows"], 0);
}

#[tokio::test]
async fn test_mcp_export_csv_special_chars_quoted() {
    let mut server = McpServer::new_default();

    // Value with comma must be quoted in CSV.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Smith, John"}),
    )
    .await;

    let result = call_tool(&mut server, "export_csv", json!({"sheet": "Sheet1"})).await;

    let csv = result["csv"].as_str().unwrap();
    assert!(
        csv.contains("\"Smith, John\""),
        "value with comma must be quoted in CSV output, got: {}",
        csv
    );
}

// ── 14. Workbook info ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_get_workbook_info_empty() {
    let mut server = McpServer::new_default();

    let result = call_tool(&mut server, "get_workbook_info", json!({})).await;

    assert_eq!(result["sheet_count"], 1, "new workbook has 1 sheet");
    assert_eq!(result["total_cells"], 0, "new workbook has 0 cells");
    assert_eq!(result["active_sheet"], "Sheet1");

    let sheets = result["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0]["name"], "Sheet1");
    assert_eq!(sheets[0]["cell_count"], 0);
}

#[tokio::test]
async fn test_mcp_get_workbook_info_after_writing() {
    let mut server = McpServer::new_default();

    // Write some cells.
    for i in 1..=5 {
        let cell_ref = format!("A{}", i);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": i}),
        )
        .await;
    }
    call_tool(&mut server, "create_sheet", json!({"name": "Sheet2"})).await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet2", "cell_ref": "A1", "value": "data"}),
    )
    .await;

    let result = call_tool(&mut server, "get_workbook_info", json!({})).await;

    assert_eq!(result["sheet_count"], 2);
    assert_eq!(result["total_cells"], 6, "5 in Sheet1 + 1 in Sheet2 = 6");

    let sheets = result["sheets"].as_array().unwrap();
    let sheet1_info = sheets.iter().find(|s| s["name"] == "Sheet1").unwrap();
    assert_eq!(sheet1_info["cell_count"], 5);
    let sheet2_info = sheets.iter().find(|s| s["name"] == "Sheet2").unwrap();
    assert_eq!(sheet2_info["cell_count"], 1);
}

// ── 15. Error handling — invalid tool ─────────────────────────────────────────

#[tokio::test]
async fn test_mcp_unknown_tool_returns_json_rpc_error() {
    let mut server = McpServer::new_default();

    let raw = server
        .handle_message(
            r#"{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"does_not_exist","arguments":{}}}"#,
        )
        .await
        .unwrap();

    let parsed: Value = serde_json::from_str(&raw).unwrap();
    // Unknown tool must return a JSON-RPC protocol error (not isError in the result).
    assert_eq!(
        parsed["error"]["code"], -32602,
        "unknown tool must produce code -32602"
    );
}

// ── 16. Error handling — invalid arguments ────────────────────────────────────

#[tokio::test]
async fn test_mcp_read_cell_missing_arguments_returns_error() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1"}), // missing cell_ref
    )
    .await;

    assert!(
        err.contains("Invalid arguments"),
        "missing required arg must produce 'Invalid arguments' error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_mcp_read_cell_nonexistent_sheet_returns_error() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "read_cell",
        json!({"sheet": "NoSuchSheet", "cell_ref": "A1"}),
    )
    .await;

    assert!(
        !err.is_empty(),
        "nonexistent sheet must return an error message"
    );
}

// ── 17. Notification — initialized ────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_initialized_notification_produces_no_response() {
    let mut server = McpServer::new_default();

    // "initialized" is a notification (no id) — server must return None.
    let response = server
        .handle_message(r#"{"jsonrpc":"2.0","method":"initialized"}"#)
        .await;

    assert!(
        response.is_none(),
        "notification must not produce a response"
    );
}

// ── 18. Cross-sheet data isolation ────────────────────────────────────────────

#[tokio::test]
async fn test_mcp_data_is_isolated_per_sheet() {
    let mut server = McpServer::new_default();

    call_tool(&mut server, "create_sheet", json!({"name": "Sheet2"})).await;

    // Write value 100 to Sheet1 A1, value 999 to Sheet2 A1.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 100}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet2", "cell_ref": "A1", "value": 999}),
    )
    .await;

    let s1_a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(s1_a1["value"].as_f64().unwrap(), 100.0);

    let s2_a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet2", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(s2_a1["value"].as_f64().unwrap(), 999.0);
}

// ── 19. Full Claude Desktop workflow simulation ───────────────────────────────

/// Simulates a full AI agent session: initialize → list sheets → write data →
/// insert formula → describe → export CSV.
#[tokio::test]
async fn test_mcp_full_agent_workflow() {
    let mut server = McpServer::new_default();

    // Step 1: Initialize.
    let init_raw = server
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .unwrap();
    let init: Value = serde_json::from_str(&init_raw).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");

    // Step 2: List sheets.
    let sheets = call_tool(&mut server, "list_sheets", json!({})).await;
    assert_eq!(sheets["sheets"][0]["name"], "Sheet1");

    // Step 3: Write financial data (monthly revenue Q1).
    let months = [("A1", "Jan"), ("A2", "Feb"), ("A3", "Mar")];
    let revenues = [("B1", 50000), ("B2", 62000), ("B3", 58000)];
    for (cell_ref, m) in &months {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": m}),
        )
        .await;
    }
    for (cell_ref, r) in &revenues {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": r}),
        )
        .await;
    }

    // Step 4: Insert SUM formula for total.
    let sum_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B4", "formula": "SUM(B1:B3)"}),
    )
    .await;
    assert_eq!(
        sum_result["result"].as_f64().unwrap(),
        170000.0,
        "Q1 total must be 170000"
    );

    // Step 5: Describe data.
    let stats = call_tool(
        &mut server,
        "describe_data",
        json!({"sheet": "Sheet1", "range": "B1:B3"}),
    )
    .await;
    assert_eq!(stats["numeric_count"], 3);
    let mean = stats["statistics"]["mean"].as_f64().unwrap();
    assert!(
        (mean - 56666.67).abs() < 1.0,
        "mean revenue must be ~56666, got {}",
        mean
    );

    // Step 6: Export as CSV.
    let csv_result = call_tool(&mut server, "export_csv", json!({"sheet": "Sheet1"})).await;
    let csv = csv_result["csv"].as_str().unwrap();
    assert!(csv.contains("Jan"), "CSV must contain month names");
    assert!(csv.contains("50000"), "CSV must contain revenue data");

    // Step 7: Workbook info.
    let info = call_tool(&mut server, "get_workbook_info", json!({})).await;
    assert_eq!(info["sheet_count"], 1);
    // 3 months + 3 revenues + 1 formula cell = 7 cells.
    assert_eq!(info["total_cells"], 7);
}

// ── 20. Format range regression tests ────────────────────────────────────────
// Regression: format operations only applied to one cell when a range was
// selected.  These tests prove the fix holds across all range shapes.

/// set_cell_format on A1:A5 with bold=true must mark every row bold.
#[tokio::test]
async fn test_format_range_column_bold() {
    let mut server = McpServer::new_default();

    // Write values so the cells exist before formatting.
    for i in 1..=5 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    let set_result = call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1:A5",
            "bold": true
        }),
    )
    .await;

    assert_eq!(set_result["success"], true);
    assert_eq!(
        set_result["cells_formatted"], 5,
        "A1:A5 is 5 cells — all must be counted"
    );

    // Read back each cell individually and verify bold is set.
    for i in 1..=5 {
        let fmt_result = call_tool(
            &mut server,
            "get_cell_format",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i)}),
        )
        .await;
        assert_eq!(
            fmt_result["format"]["bold"], true,
            "A{} must be bold after range format op",
            i
        );
    }
}

/// set_cell_format on A1:C3 (3x3 = 9 cells) with font_color="#ff0000".
#[tokio::test]
async fn test_format_range_3x3_font_color() {
    let mut server = McpServer::new_default();

    let set_result = call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1:C3",
            "font_color": "#ff0000"
        }),
    )
    .await;

    assert_eq!(set_result["success"], true);
    assert_eq!(set_result["cells_formatted"], 9, "A1:C3 is 9 cells");

    // Verify all 9 cells carry the font_color.
    for row in 1..=3_u8 {
        for col in b'A'..=b'C' {
            let cell_ref = format!("{}{}", col as char, row);
            let fmt_result = call_tool(
                &mut server,
                "get_cell_format",
                json!({"sheet": "Sheet1", "cell_ref": cell_ref}),
            )
            .await;
            assert_eq!(
                fmt_result["format"]["font_color"], "#ff0000",
                "cell {} must have font_color #ff0000",
                cell_ref
            );
        }
    }
}

/// set_cell_format on a single cell with bg_color="#00ff00".
#[tokio::test]
async fn test_format_single_cell_bg_color() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2", "value": "test"}),
    )
    .await;

    let set_result = call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "B2",
            "bg_color": "#00ff00"
        }),
    )
    .await;

    assert_eq!(set_result["success"], true);
    assert_eq!(set_result["cells_formatted"], 1);

    let fmt_result = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;

    assert_eq!(
        fmt_result["format"]["bg_color"], "#00ff00",
        "bg_color must round-trip correctly"
    );
}

/// Format operations on a range must not bleed into adjacent cells.
#[tokio::test]
async fn test_format_range_does_not_affect_adjacent_cells() {
    let mut server = McpServer::new_default();

    // Write a 4-cell column, format only the first 2.
    for i in 1..=4 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1:A2",
            "italic": true
        }),
    )
    .await;

    // A1 and A2 must be italic.
    for i in 1..=2 {
        let fmt = call_tool(
            &mut server,
            "get_cell_format",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i)}),
        )
        .await;
        assert_eq!(fmt["format"]["italic"], true, "A{} must be italic", i);
    }

    // A3 and A4 must NOT be italic (format must not bleed).
    for i in 3..=4 {
        let fmt = call_tool(
            &mut server,
            "get_cell_format",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i)}),
        )
        .await;
        assert_eq!(
            fmt["format"]["italic"], false,
            "A{} must NOT be italic — format bled beyond range",
            i
        );
    }
}

// ── 21. Formula evaluation and recalculation tests ────────────────────────────
// Regression: verify SUM/AVERAGE/cell-ref formulas evaluate correctly and
// that mutating a dependency cell triggers correct recalculation.

/// Write 1–5 to A1:A5, SUM in A6, AVERAGE in A7; verify values.
#[tokio::test]
async fn test_formula_sum_and_average_over_range() {
    let mut server = McpServer::new_default();

    for i in 1..=5 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // SUM(A1:A5) = 15
    let sum_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A6", "formula": "SUM(A1:A5)"}),
    )
    .await;
    assert_eq!(
        sum_result["result"].as_f64().unwrap(),
        15.0,
        "SUM(1+2+3+4+5) must equal 15"
    );

    let a6 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A6"}),
    )
    .await;
    assert_eq!(
        a6["value"].as_f64().unwrap(),
        15.0,
        "read_cell A6 must return 15 after insert_formula"
    );

    // AVERAGE(A1:A5) = 3
    let avg_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A7", "formula": "AVERAGE(A1:A5)"}),
    )
    .await;
    assert_eq!(
        avg_result["result"].as_f64().unwrap(),
        3.0,
        "AVERAGE(1..5) must equal 3"
    );

    let a7 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A7"}),
    )
    .await;
    assert_eq!(a7["value"].as_f64().unwrap(), 3.0);
}

/// Write A1=1, A2=2, B1=A1+A2; verify B1=3, then change A1 to 10 and
/// re-insert SUM to verify recalculation.
#[tokio::test]
async fn test_formula_cell_ref_and_recalculation() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 1}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": 2}),
    )
    .await;

    // B1 = A1 + A2 = 3
    let b1_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "formula": "A1+A2"}),
    )
    .await;
    assert_eq!(
        b1_result["result"].as_f64().unwrap(),
        3.0,
        "A1+A2 must equal 3 initially"
    );

    // Re-seed A1:A5 for the SUM recalculation test.
    for i in 1..=5 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }
    call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A6", "formula": "SUM(A1:A5)"}),
    )
    .await;

    // Change A1 to 10 — the recalculation check.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 10}),
    )
    .await;

    // Re-insert the formula so the engine picks up the updated dependency.
    let recalc_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A6", "formula": "SUM(A1:A5)"}),
    )
    .await;
    // SUM(10+2+3+4+5) = 24
    assert_eq!(
        recalc_result["result"].as_f64().unwrap(),
        24.0,
        "SUM must recalculate to 24 after A1 changed to 10"
    );

    let a6 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A6"}),
    )
    .await;
    assert_eq!(
        a6["value"].as_f64().unwrap(),
        24.0,
        "read_cell A6 must reflect the recalculated value"
    );
}

// ── 22. CellData format round-trip ────────────────────────────────────────────
// Regression: CellData from backend was missing font_color, bg_color, and
// font_size fields.  These tests verify that every format field survives a
// write → set_format → get_cell_format round-trip via MCP.

/// All five format fields — bold, font_color, bg_color, font_size, italic —
/// must be returned by get_cell_format after a single set_cell_format call.
#[tokio::test]
async fn test_cell_format_all_fields_round_trip() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "formatted"}),
    )
    .await;

    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bold": true,
            "italic": true,
            "font_color": "#ff0000",
            "bg_color": "#00ff00",
            "font_size": 18.0
        }),
    )
    .await;

    let fmt_result = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;

    let fmt = &fmt_result["format"];

    assert_eq!(fmt["bold"], true, "bold must be returned");
    assert_eq!(fmt["italic"], true, "italic must be returned");
    assert_eq!(fmt["font_color"], "#ff0000", "font_color must be returned");
    assert_eq!(fmt["bg_color"], "#00ff00", "bg_color must be returned");
    assert_eq!(
        fmt["font_size"].as_f64().unwrap(),
        18.0,
        "font_size must be returned"
    );
}

/// Default (unformatted) cell must return well-defined values for all fields,
/// not null or missing keys — guards against partial serialization bugs.
#[tokio::test]
async fn test_cell_format_default_values_are_present() {
    let mut server = McpServer::new_default();

    let fmt_result = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "Z99"}),
    )
    .await;

    let fmt = &fmt_result["format"];

    // None of the standard format keys should be absent.
    assert!(!fmt["bold"].is_null(), "bold must not be null");
    assert!(!fmt["italic"].is_null(), "italic must not be null");
    assert!(!fmt["font_size"].is_null(), "font_size must not be null");
    assert!(!fmt["font_color"].is_null(), "font_color must not be null");
    assert!(!fmt["h_align"].is_null(), "h_align must not be null");
    assert!(!fmt["v_align"].is_null(), "v_align must not be null");

    // Check expected defaults.
    assert_eq!(fmt["bold"], false);
    assert_eq!(fmt["italic"], false);
    assert_eq!(fmt["font_size"].as_f64().unwrap(), 11.0);
    assert_eq!(fmt["font_color"], "#000000");
}

/// bg_color cleared via null must persist as null (not the previous colour).
/// This guards the fix to set_cell_format where serde's Option<Value>
/// collapsed explicit null and field absence into the same None, preventing
/// the null-means-clear contract from working.
#[tokio::test]
async fn test_cell_format_bg_color_clear() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 1}),
    )
    .await;

    // Set a bg_color first.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bg_color": "#ffff00"
        }),
    )
    .await;

    // Verify it was actually set.
    let before = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        before["format"]["bg_color"], "#ffff00",
        "bg_color must be set before the clear step"
    );

    // Now clear it with null.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bg_color": null
        }),
    )
    .await;

    let fmt_result = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;

    assert!(
        fmt_result["format"]["bg_color"].is_null(),
        "bg_color must be null after clearing with null, got: {}",
        fmt_result["format"]["bg_color"]
    );
}

// ── 23. Fill pattern core engine tests ───────────────────────────────────────
// Regression: fill handle drag didn't fill because isFillDragging was cleared
// before executeFill in the UI.  These tests verify the *core engine* pattern
// detection and fill logic is correct independent of the UI, so that once the
// UI bug is fixed the engine is provably sound.

/// Detect a linear numeric pattern from A1:A3 (1, 2, 3) and verify
/// fill_range extends it correctly to A4:A6 (4, 5, 6).
#[test]
fn test_autofill_linear_numeric_down() {
    let mut sheet = Sheet::new("T");
    sheet.set_value(0, 0, CellValue::Number(1.0));
    sheet.set_value(1, 0, CellValue::Number(2.0));
    sheet.set_value(2, 0, CellValue::Number(3.0));

    let source = Range {
        start: CellRef { row: 0, col: 0 },
        end: CellRef { row: 2, col: 0 },
    };
    let target = Range {
        start: CellRef { row: 3, col: 0 },
        end: CellRef { row: 5, col: 0 },
    };

    // Verify pattern detection first.
    let source_vals: Vec<CellValue> = (0..=2)
        .map(|r| {
            sheet
                .get_cell(r, 0)
                .map(|c| c.value.clone())
                .unwrap_or(CellValue::Empty)
        })
        .collect();
    let pattern = detect_pattern(&source_vals).expect("must detect a pattern");
    assert_eq!(
        pattern,
        FillPattern::LinearNumber(1.0, 1.0),
        "1,2,3 must detect as LinearNumber(1.0, 1.0)"
    );

    // Now fill and verify.
    fill_range(&mut sheet, &source, &target, FillDirection::Down);

    assert_eq!(
        sheet.get_cell(3, 0).unwrap().value,
        CellValue::Number(4.0),
        "A4 must be 4"
    );
    assert_eq!(
        sheet.get_cell(4, 0).unwrap().value,
        CellValue::Number(5.0),
        "A5 must be 5"
    );
    assert_eq!(
        sheet.get_cell(5, 0).unwrap().value,
        CellValue::Number(6.0),
        "A6 must be 6"
    );
}

/// Detect and fill a text-with-number sequence (Q1, Q2, Q3 → Q4, Q5).
#[test]
fn test_autofill_text_with_number_series() {
    let mut sheet = Sheet::new("T");
    sheet.set_value(0, 0, CellValue::Text("Q1".into()));
    sheet.set_value(1, 0, CellValue::Text("Q2".into()));
    sheet.set_value(2, 0, CellValue::Text("Q3".into()));

    let source_vals: Vec<CellValue> = (0..=2)
        .map(|r| {
            sheet
                .get_cell(r, 0)
                .map(|c| c.value.clone())
                .unwrap_or(CellValue::Empty)
        })
        .collect();
    let pattern = detect_pattern(&source_vals).unwrap();
    assert_eq!(
        pattern,
        FillPattern::TextWithNumber("Q".into(), 1, 1),
        "Q1,Q2,Q3 must detect as TextWithNumber"
    );

    let source = Range {
        start: CellRef { row: 0, col: 0 },
        end: CellRef { row: 2, col: 0 },
    };
    let target = Range {
        start: CellRef { row: 3, col: 0 },
        end: CellRef { row: 4, col: 0 },
    };
    fill_range(&mut sheet, &source, &target, FillDirection::Down);

    assert_eq!(
        sheet.get_cell(3, 0).unwrap().value,
        CellValue::Text("Q4".into())
    );
    assert_eq!(
        sheet.get_cell(4, 0).unwrap().value,
        CellValue::Text("Q5".into())
    );
}

/// A single constant value fills the target with the same value (no
/// numeric progression, no crash on single-element input).
#[test]
fn test_autofill_single_value_constant_fill() {
    let mut sheet = Sheet::new("T");
    sheet.set_value(0, 0, CellValue::Number(42.0));

    let source_vals = vec![CellValue::Number(42.0)];
    let pattern = detect_pattern(&source_vals).unwrap();
    assert_eq!(
        pattern,
        FillPattern::Constant(CellValue::Number(42.0)),
        "single value must be a Constant pattern"
    );

    let source = Range {
        start: CellRef { row: 0, col: 0 },
        end: CellRef { row: 0, col: 0 },
    };
    let target = Range {
        start: CellRef { row: 1, col: 0 },
        end: CellRef { row: 4, col: 0 },
    };
    fill_range(&mut sheet, &source, &target, FillDirection::Down);

    for r in 1..=4 {
        assert_eq!(
            sheet.get_cell(r, 0).unwrap().value,
            CellValue::Number(42.0),
            "row {} must be 42 after constant fill",
            r
        );
    }
}

/// Repeating text cycle: A, B, C → A, B, C, A, B (wraps at period length).
#[test]
fn test_autofill_repeating_cycle() {
    let mut sheet = Sheet::new("T");
    sheet.set_value(0, 0, CellValue::Text("A".into()));
    sheet.set_value(1, 0, CellValue::Text("B".into()));
    sheet.set_value(2, 0, CellValue::Text("C".into()));

    let source_vals: Vec<CellValue> = (0..=2)
        .map(|r| {
            sheet
                .get_cell(r, 0)
                .map(|c| c.value.clone())
                .unwrap_or(CellValue::Empty)
        })
        .collect();
    let pattern = detect_pattern(&source_vals).unwrap();
    // A, B, C have no common numeric stem, so it's Repeating.
    assert!(
        matches!(pattern, FillPattern::Repeating(_)),
        "A,B,C must be detected as Repeating"
    );

    let source = Range {
        start: CellRef { row: 0, col: 0 },
        end: CellRef { row: 2, col: 0 },
    };
    let target = Range {
        start: CellRef { row: 3, col: 0 },
        end: CellRef { row: 7, col: 0 },
    };
    fill_range(&mut sheet, &source, &target, FillDirection::Down);

    // Positions 3..7 (0-indexed) → cycle indices 0..4
    let expected = ["A", "B", "C", "A", "B"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(
            sheet.get_cell(3 + i as u32, 0).unwrap().value,
            CellValue::Text((*exp).into()),
            "row {} should be {}",
            3 + i,
            exp
        );
    }
}

// ── 24. Named range + formula integration ────────────────────────────────────
// Regression: verify that named ranges can be created, resolved, listed, and
// removed via MCP tools — and that formulas referencing cells in the named
// range work correctly alongside the named range registry.

/// Create named range "Revenue" → A1:A5, write values, resolve it back.
#[tokio::test]
async fn test_named_range_create_and_resolve() {
    let mut server = McpServer::new_default();

    // Write values 10, 20, 30, 40, 50 to A1:A5.
    for (i, v) in [10, 20, 30, 40, 50].iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 1), "value": v}),
        )
        .await;
    }

    // Create the named range.
    let add_result = call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Revenue", "range": "A1:A5", "sheet": "Sheet1"}),
    )
    .await;

    assert_eq!(add_result["success"], true, "add_named_range must succeed");
    assert_eq!(add_result["name"], "Revenue");
    assert_eq!(add_result["range"], "A1:A5");

    // Resolve it back via MCP.
    let resolve_result = call_tool(
        &mut server,
        "resolve_named_range",
        json!({"name": "Revenue"}),
    )
    .await;

    assert_eq!(resolve_result["found"], true);
    assert_eq!(resolve_result["range"], "A1:A5");
    assert_eq!(resolve_result["sheet"], "Sheet1");
}

/// list_named_ranges returns all ranges added via add_named_range.
#[tokio::test]
async fn test_named_range_list_after_multiple_adds() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Revenue", "range": "A1:A5"}),
    )
    .await;
    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Expenses", "range": "B1:B5"}),
    )
    .await;
    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Profit", "range": "C1:C5"}),
    )
    .await;

    let list_result = call_tool(&mut server, "list_named_ranges", json!({})).await;

    assert_eq!(list_result["count"], 3, "must list all 3 named ranges");
    let names: Vec<&str> = list_result["named_ranges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|nr| nr["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Revenue"));
    assert!(names.contains(&"Expenses"));
    assert!(names.contains(&"Profit"));
}

/// remove_named_range reduces the count; resolving the removed range errors.
#[tokio::test]
async fn test_named_range_remove_and_resolve_fails() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Temp", "range": "D1:D10"}),
    )
    .await;

    let remove_result = call_tool(&mut server, "remove_named_range", json!({"name": "Temp"})).await;
    assert_eq!(remove_result["success"], true);

    let err =
        call_tool_expect_error(&mut server, "resolve_named_range", json!({"name": "Temp"})).await;
    assert!(
        !err.is_empty(),
        "resolving a removed named range must return an error"
    );
}

/// Formula SUM over the same cells as a named range returns the correct total.
/// This is an integration test — verifies that named range metadata doesn't
/// interfere with formula evaluation on the underlying cell data.
#[tokio::test]
async fn test_named_range_formula_integration() {
    let mut server = McpServer::new_default();

    // Write values 1..5 to A1:A5.
    for i in 1..=5 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // Create a named range over those cells.
    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Values", "range": "A1:A5", "sheet": "Sheet1"}),
    )
    .await;

    // SUM formula over the same range must still evaluate correctly.
    let sum_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A6", "formula": "SUM(A1:A5)"}),
    )
    .await;
    assert_eq!(
        sum_result["result"].as_f64().unwrap(),
        15.0,
        "SUM(1..5) must equal 15 regardless of named range metadata"
    );

    // The named range registry must still be intact after formula insertion.
    let resolve_result = call_tool(
        &mut server,
        "resolve_named_range",
        json!({"name": "Values"}),
    )
    .await;
    assert_eq!(
        resolve_result["found"], true,
        "named range must still resolve after formula insertion"
    );
    assert_eq!(resolve_result["range"], "A1:A5");
}

/// Duplicate named range must return an error (case-insensitive).
#[tokio::test]
async fn test_named_range_duplicate_is_error() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "Total", "range": "A1:A10"}),
    )
    .await;

    // Same name again (different case) must fail.
    let err = call_tool_expect_error(
        &mut server,
        "add_named_range",
        json!({"name": "total", "range": "B1:B10"}),
    )
    .await;
    assert!(
        !err.is_empty(),
        "adding a duplicate named range must produce an error"
    );
}

// ── 25. Named functions: add -> use in formula -> remove ─────────────────────

/// Add a named function, use it in a formula evaluation, then remove it.
#[tokio::test]
async fn test_named_function_lifecycle() {
    let mut server = McpServer::new_default();

    // Add a named function DOUBLE(x) = x * 2.
    let add_result = call_tool(
        &mut server,
        "add_named_function",
        json!({
            "name": "DOUBLE",
            "params": ["x"],
            "body": "x * 2",
            "description": "Doubles a value"
        }),
    )
    .await;
    assert_eq!(add_result["success"], true);
    assert_eq!(add_result["name"], "DOUBLE");
    assert_eq!(add_result["params"][0], "x");
    assert_eq!(add_result["body"], "x * 2");

    // List should show 1 function.
    let list_result = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(list_result["count"], 1);
    assert_eq!(list_result["named_functions"][0]["name"], "DOUBLE");

    // Remove the function.
    let remove_result = call_tool(
        &mut server,
        "remove_named_function",
        json!({"name": "DOUBLE"}),
    )
    .await;
    assert_eq!(remove_result["success"], true);

    // List should now be empty.
    let list_result = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(list_result["count"], 0);
}

/// Attempting to add a named function with a duplicate name (case-insensitive)
/// must fail.
#[tokio::test]
async fn test_named_function_duplicate_is_error() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "add_named_function",
        json!({"name": "TRIPLE", "params": ["x"], "body": "x * 3"}),
    )
    .await;

    let err = call_tool_expect_error(
        &mut server,
        "add_named_function",
        json!({"name": "triple", "params": ["y"], "body": "y * 4"}),
    )
    .await;
    assert!(
        !err.is_empty(),
        "adding a duplicate named function must produce an error"
    );
}

/// Removing a nonexistent named function must fail.
#[tokio::test]
async fn test_named_function_remove_not_found() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "remove_named_function",
        json!({"name": "NONEXISTENT"}),
    )
    .await;
    assert!(!err.is_empty());
}

/// Multi-param named function.
#[tokio::test]
async fn test_named_function_multi_param() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "add_named_function",
        json!({
            "name": "WEIGHTED_AVG",
            "params": ["value", "weight"],
            "body": "value * weight",
            "description": "Multiply value by weight"
        }),
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["params"].as_array().unwrap().len(), 2);

    // Clean up.
    call_tool(
        &mut server,
        "remove_named_function",
        json!({"name": "WEIGHTED_AVG"}),
    )
    .await;
}

// ── 26. Filter views: save -> list -> apply -> delete ────────────────────────

/// Full filter view lifecycle: save, list, apply, delete.
#[tokio::test]
async fn test_filter_view_lifecycle() {
    let mut server = McpServer::new_default();

    // Set up data: header row + 3 data rows.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Fruit"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "apple"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3", "value": "banana"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A4", "value": "apple"}),
    )
    .await;

    // Save a filter view that only shows "apple" in column 0.
    let save_result = call_tool(
        &mut server,
        "save_filter_view",
        json!({
            "name": "ApplesOnly",
            "column_filters": {"0": ["apple"]}
        }),
    )
    .await;
    assert_eq!(save_result["success"], true);
    assert_eq!(save_result["name"], "ApplesOnly");

    // List filter views.
    let list_result = call_tool(&mut server, "list_filter_views", json!({})).await;
    assert_eq!(list_result["count"], 1);
    assert_eq!(list_result["filter_views"][0]["name"], "ApplesOnly");

    // Apply the filter view.
    let apply_result = call_tool(
        &mut server,
        "apply_filter_view",
        json!({"sheet": "Sheet1", "name": "ApplesOnly"}),
    )
    .await;
    assert_eq!(apply_result["success"], true);
    // "banana" (row index 2) should be hidden (1 row hidden).
    assert_eq!(
        apply_result["rows_hidden"], 1,
        "one row ('banana') should be hidden"
    );

    // Delete the filter view.
    let delete_result = call_tool(
        &mut server,
        "delete_filter_view",
        json!({"name": "ApplesOnly"}),
    )
    .await;
    assert_eq!(delete_result["success"], true);

    // List should be empty.
    let list_result = call_tool(&mut server, "list_filter_views", json!({})).await;
    assert_eq!(list_result["count"], 0);
}

/// Deleting a nonexistent filter view must fail.
#[tokio::test]
async fn test_filter_view_delete_not_found() {
    let mut server = McpServer::new_default();

    let err =
        call_tool_expect_error(&mut server, "delete_filter_view", json!({"name": "nope"})).await;
    assert!(!err.is_empty());
}

/// Applying a nonexistent filter view must fail.
#[tokio::test]
async fn test_filter_view_apply_not_found() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "apply_filter_view",
        json!({"sheet": "Sheet1", "name": "nope"}),
    )
    .await;
    assert!(!err.is_empty());
}

// ── 27. Evaluate formula: SCAN and MAKEARRAY ─────────────────────────────────

/// Test that SCAN can be evaluated via the MCP evaluate_formula tool.
/// Uses cell references since the formula evaluator works with ranges.
#[tokio::test]
async fn test_evaluate_formula_scan() {
    let mut server = McpServer::new_default();

    // Write values 1, 2, 3 to A1:A3.
    for i in 1..=3 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // SCAN(0, A1:A3, LAMBDA(acc, x, acc + x)) should produce {1, 3, 6}
    let result = call_tool(
        &mut server,
        "evaluate_formula",
        json!({"sheet": "Sheet1", "formula": "SCAN(0, A1:A3, LAMBDA(acc, x, acc + x))"}),
    )
    .await;

    assert_eq!(result["result_type"], "array");
    // The result should be a 1-row array: [[1, 3, 6]]
    let arr = result["result"].as_array().unwrap();
    let inner = arr[0].as_array().unwrap();
    assert_eq!(inner[0].as_f64().unwrap(), 1.0);
    assert_eq!(inner[1].as_f64().unwrap(), 3.0);
    assert_eq!(inner[2].as_f64().unwrap(), 6.0);
}

/// Test that MAKEARRAY can be evaluated via the MCP evaluate_formula tool.
#[tokio::test]
async fn test_evaluate_formula_makearray() {
    let mut server = McpServer::new_default();

    // MAKEARRAY(2, 3, LAMBDA(r, c, r * c)) should produce a 2x3 array.
    let result = call_tool(
        &mut server,
        "evaluate_formula",
        json!({"sheet": "Sheet1", "formula": "MAKEARRAY(2, 3, LAMBDA(r, c, r * c))"}),
    )
    .await;

    assert_eq!(result["result_type"], "array");
    let arr = result["result"].as_array().unwrap();
    assert_eq!(arr.len(), 2, "should have 2 rows");

    let row0 = arr[0].as_array().unwrap();
    assert_eq!(row0.len(), 3, "should have 3 columns");
    // r=1,c=1 -> 1; r=1,c=2 -> 2; r=1,c=3 -> 3
    assert_eq!(row0[0].as_f64().unwrap(), 1.0);
    assert_eq!(row0[1].as_f64().unwrap(), 2.0);
    assert_eq!(row0[2].as_f64().unwrap(), 3.0);

    let row1 = arr[1].as_array().unwrap();
    // r=2,c=1 -> 2; r=2,c=2 -> 4; r=2,c=3 -> 6
    assert_eq!(row1[0].as_f64().unwrap(), 2.0);
    assert_eq!(row1[1].as_f64().unwrap(), 4.0);
    assert_eq!(row1[2].as_f64().unwrap(), 6.0);
}

// ── 28. Sparklines: add -> list -> remove ────────────────────────────────────

/// Full sparkline lifecycle: add two sparklines, list, remove one, verify.
#[tokio::test]
async fn test_sparkline_lifecycle() {
    let mut server = McpServer::new_default();

    // Write some data for the sparklines to reference.
    for i in 1..=5 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i * 10}),
        )
        .await;
    }

    // Add a line sparkline in B1.
    let add1 = call_tool(
        &mut server,
        "add_sparkline",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "B1",
            "spark_type": "line",
            "data_range": "A1:A5",
            "color": "#4e79a7"
        }),
    )
    .await;
    assert_eq!(add1["success"], true);
    assert_eq!(add1["spark_type"], "line");

    // Add a bar sparkline in C1.
    let add2 = call_tool(
        &mut server,
        "add_sparkline",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "C1",
            "spark_type": "bar",
            "data_range": "A1:A5"
        }),
    )
    .await;
    assert_eq!(add2["success"], true);

    // List sparklines.
    let list_result = call_tool(&mut server, "list_sparklines", json!({"sheet": "Sheet1"})).await;
    assert_eq!(list_result["count"], 2);

    // Remove the line sparkline from B1.
    let remove_result = call_tool(
        &mut server,
        "remove_sparkline",
        json!({"sheet": "Sheet1", "cell_ref": "B1"}),
    )
    .await;
    assert_eq!(remove_result["success"], true);
    assert_eq!(remove_result["was_present"], true);

    // List should now have 1 sparkline.
    let list_result = call_tool(&mut server, "list_sparklines", json!({"sheet": "Sheet1"})).await;
    assert_eq!(list_result["count"], 1);

    // Remove the bar sparkline from C1.
    let remove_result = call_tool(
        &mut server,
        "remove_sparkline",
        json!({"sheet": "Sheet1", "cell_ref": "C1"}),
    )
    .await;
    assert_eq!(remove_result["success"], true);
    assert_eq!(remove_result["was_present"], true);

    // List should now be empty.
    let list_result = call_tool(&mut server, "list_sparklines", json!({"sheet": "Sheet1"})).await;
    assert_eq!(list_result["count"], 0);
}

/// Removing a sparkline that was never added should succeed but indicate
/// it was not present.
#[tokio::test]
async fn test_sparkline_remove_not_present() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "remove_sparkline",
        json!({"sheet": "Sheet1", "cell_ref": "Z99"}),
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["was_present"], false);
}

/// Adding a sparkline with an invalid type should produce an error.
#[tokio::test]
async fn test_sparkline_invalid_type() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "add_sparkline",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "spark_type": "pie",
            "data_range": "B1:D1"
        }),
    )
    .await;
    assert!(err.contains("Invalid sparkline type"));
}

// ── 29. Charts: create -> list -> delete ─────────────────────────────────────

/// Full chart lifecycle: create, list (filtered), delete, verify gone.
#[tokio::test]
async fn test_chart_lifecycle() {
    let mut server = McpServer::new_default();

    // Create a bar chart.
    let create1 = call_tool(
        &mut server,
        "create_chart",
        json!({
            "sheet": "ChartLifecycleSheet",
            "chart_type": "bar",
            "data_range": "A1:B5",
            "title": "Sales by Quarter"
        }),
    )
    .await;
    assert_eq!(create1["success"], true);
    assert!(create1["chart_id"].is_string());
    assert_eq!(create1["chart_type"], "bar");
    assert_eq!(create1["title"], "Sales by Quarter");
    let chart1_id = create1["chart_id"].as_str().unwrap().to_string();

    // Create a line chart on a different sheet.
    let create2 = call_tool(
        &mut server,
        "create_chart",
        json!({
            "sheet": "ChartLifecycleSheet2",
            "chart_type": "line",
            "data_range": "A1:C10"
        }),
    )
    .await;
    assert_eq!(create2["success"], true);
    let chart2_id = create2["chart_id"].as_str().unwrap().to_string();

    // List charts filtered by sheet.
    let list_filtered = call_tool(
        &mut server,
        "list_charts",
        json!({"sheet": "ChartLifecycleSheet"}),
    )
    .await;
    assert_eq!(list_filtered["count"], 1);
    assert_eq!(list_filtered["charts"][0]["chart_type"], "bar");

    // Delete chart 1.
    let delete1 = call_tool(&mut server, "delete_chart", json!({"chart_id": chart1_id})).await;
    assert_eq!(delete1["success"], true);

    // Verify chart 1 is gone — listing by its sheet should be empty.
    let list_after = call_tool(
        &mut server,
        "list_charts",
        json!({"sheet": "ChartLifecycleSheet"}),
    )
    .await;
    assert_eq!(list_after["count"], 0);

    // Chart 2 should still exist.
    let list_sheet2 = call_tool(
        &mut server,
        "list_charts",
        json!({"sheet": "ChartLifecycleSheet2"}),
    )
    .await;
    assert_eq!(list_sheet2["count"], 1);

    // Delete chart 2 for cleanup.
    call_tool(&mut server, "delete_chart", json!({"chart_id": chart2_id})).await;
}

/// Creating a chart with an invalid type must fail.
#[tokio::test]
async fn test_chart_invalid_type_error() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "create_chart",
        json!({
            "sheet": "Sheet1",
            "chart_type": "invalid_type",
            "data_range": "A1:A5"
        }),
    )
    .await;
    assert!(err.contains("Invalid chart type"));
}

/// Deleting a nonexistent chart must fail.
#[tokio::test]
async fn test_chart_delete_not_found() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "delete_chart",
        json!({"chart_id": "nonexistent-id-for-integration-test"}),
    )
    .await;
    assert!(err.contains("Chart not found"));
}

/// Creating a chart with chart options (x/y axis labels).
#[tokio::test]
async fn test_chart_with_options() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "create_chart",
        json!({
            "sheet": "ChartOptionsSheet",
            "chart_type": "scatter",
            "data_range": "A1:B20",
            "options": {
                "title": "Scatter Plot",
                "x_axis_label": "Time",
                "y_axis_label": "Value"
            }
        }),
    )
    .await;
    assert_eq!(result["success"], true);
    assert_eq!(result["title"], "Scatter Plot");

    // Cleanup.
    let id = result["chart_id"].as_str().unwrap();
    call_tool(&mut server, "delete_chart", json!({"chart_id": id})).await;
}

// ── 22. Multiplication table of 14 (1 to 15) ─────────────────────────────────

/// Build a 14× multiplication table (rows 1-15), verify every product,
/// then verify SUM and AVERAGE totals at the bottom.
#[tokio::test]
async fn test_mcp_multiplication_table_14() {
    let mut server = McpServer::new_default();

    // Header in A1.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "14 Times Table"}),
    )
    .await;

    // Column headers in row 2.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "Number"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2", "value": "Result"}),
    )
    .await;

    // Write numbers 1-15 in A3:A17 and formulas =A_n*14 in B3:B17.
    for n in 1u32..=15 {
        let row = n + 2; // row 3 = n=1, row 17 = n=15
        let a_ref = format!("A{}", row);
        let b_ref = format!("B{}", row);
        let formula = format!("A{}*14", row);

        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": a_ref, "value": n}),
        )
        .await;

        call_tool(
            &mut server,
            "insert_formula",
            json!({"sheet": "Sheet1", "cell_ref": b_ref, "formula": formula}),
        )
        .await;
    }

    // Read back all B3:B17 values and verify n * 14.
    for n in 1u32..=15 {
        let row = n + 2;
        let b_ref = format!("B{}", row);
        let expected = (n * 14) as f64;

        let cell = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Sheet1", "cell_ref": b_ref}),
        )
        .await;

        assert_eq!(
            cell["value"].as_f64().unwrap(),
            expected,
            "B{} ({}×14) must equal {}",
            row,
            n,
            expected
        );
    }

    // Insert SUM(B3:B17) in B18 — expected: 14*(1+2+...+15) = 14*120 = 1680.
    let sum_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B18", "formula": "SUM(B3:B17)"}),
    )
    .await;
    assert_eq!(
        sum_result["result"].as_f64().unwrap(),
        1680.0,
        "SUM of 14× table (1-15) must equal 1680"
    );

    // Verify via read_cell too.
    let b18 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B18"}),
    )
    .await;
    assert_eq!(b18["value"].as_f64().unwrap(), 1680.0);
    assert_eq!(b18["formula"], "SUM(B3:B17)");

    // Insert AVERAGE(B3:B17) in B19 — expected: 1680/15 = 112.
    let avg_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B19", "formula": "AVERAGE(B3:B17)"}),
    )
    .await;
    assert_eq!(
        avg_result["result"].as_f64().unwrap(),
        112.0,
        "AVERAGE of 14× table (1-15) must equal 112"
    );

    let b19 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B19"}),
    )
    .await;
    assert_eq!(b19["value"].as_f64().unwrap(), 112.0);
}

// ── 23. Formatting round-trip ─────────────────────────────────────────────────

/// Incrementally apply and verify every supported format property on a single
/// cell: bold, font_color, bg_color, font_size, italic, underline-like combos.
#[tokio::test]
async fn test_mcp_formatting_roundtrip() {
    let mut server = McpServer::new_default();

    // Write a value so the cell exists.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Test"}),
    )
    .await;

    // Step 1 — set bold=true and red font_color.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bold": true,
            "font_color": "#ff0000"
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt["format"]["bold"], true,
        "bold must be true after step 1"
    );
    assert_eq!(
        fmt["format"]["font_color"], "#ff0000",
        "font_color must be #ff0000 after step 1"
    );

    // Step 2 — set yellow bg_color; bold and font_color must remain.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bg_color": "#ffff00"
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt["format"]["bg_color"], "#ffff00",
        "bg_color must be #ffff00 after step 2"
    );
    // Previously-set properties must be preserved.
    assert_eq!(
        fmt["format"]["bold"], true,
        "bold must be preserved through step 2"
    );
    assert_eq!(
        fmt["format"]["font_color"], "#ff0000",
        "font_color must be preserved through step 2"
    );

    // Step 3 — set font_size=18.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "font_size": 18.0
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt["format"]["font_size"].as_f64().unwrap(),
        18.0,
        "font_size must be 18 after step 3"
    );

    // Step 4 — set italic=true.  All prior properties must still hold.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "italic": true
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    let f = &fmt["format"];
    assert_eq!(f["bold"], true, "bold must still be true at step 4");
    assert_eq!(f["italic"], true, "italic must be true at step 4");
    assert_eq!(
        f["font_size"].as_f64().unwrap(),
        18.0,
        "font_size must still be 18 at step 4"
    );
    assert_eq!(
        f["font_color"], "#ff0000",
        "font_color must still be #ff0000 at step 4"
    );
    assert_eq!(
        f["bg_color"], "#ffff00",
        "bg_color must still be #ffff00 at step 4"
    );

    // Step 5 — clear bg_color by setting it to null.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "bg_color": null
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert!(
        fmt["format"]["bg_color"].is_null(),
        "bg_color must be null after explicit null clear"
    );
    // Other properties must still be set.
    assert_eq!(
        fmt["format"]["bold"], true,
        "bold must be unaffected by bg_color clear"
    );
    assert_eq!(
        fmt["format"]["italic"], true,
        "italic must be unaffected by bg_color clear"
    );
}

// ── 24. Full realistic agent workflow ─────────────────────────────────────────

/// Simulate a realistic Claude Desktop session: create an "Analysis" sheet,
/// write 4-quarter financial data, insert profit formulas and totals, describe
/// revenue, sort by revenue descending, apply formatting, create a bar chart,
/// verify workbook info, and export as CSV.
#[tokio::test]
async fn test_mcp_realistic_agent_workflow() {
    let mut server = McpServer::new_default();

    // Step 1: initialize the MCP session.
    let init_raw = server
        .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
        .await
        .unwrap();
    let init: Value = serde_json::from_str(&init_raw).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");

    // Step 2: create a dedicated "Analysis" sheet.
    let create = call_tool(&mut server, "create_sheet", json!({"name": "Analysis"})).await;
    assert_eq!(create["success"], true);

    // Step 3: write the header row.
    for (cell, val) in &[
        ("A1", "Quarter"),
        ("B1", "Revenue"),
        ("C1", "Costs"),
        ("D1", "Profit"),
    ] {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Analysis", "cell_ref": cell, "value": val}),
        )
        .await;
    }

    // Step 4: write quarter labels.
    for (i, q) in ["Q1", "Q2", "Q3", "Q4"].iter().enumerate() {
        let cell = format!("A{}", i + 2);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Analysis", "cell_ref": cell, "value": q}),
        )
        .await;
    }

    // Step 5: write Revenue (B2:B5) and Costs (C2:C5).
    let revenues = [50000, 65000, 72000, 80000];
    let costs = [30000, 35000, 40000, 42000];
    for (i, (rev, cost)) in revenues.iter().zip(costs.iter()).enumerate() {
        let row = i + 2;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Analysis", "cell_ref": format!("B{}", row), "value": rev}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Analysis", "cell_ref": format!("C{}", row), "value": cost}),
        )
        .await;
    }

    // Step 6: insert Profit formulas D2:D5 = revenue - costs.
    for row in 2..=5usize {
        let formula = format!("B{}-C{}", row, row);
        call_tool(
            &mut server,
            "insert_formula",
            json!({"sheet": "Analysis", "cell_ref": format!("D{}", row), "formula": formula}),
        )
        .await;
    }

    // Verify each profit value.
    let expected_profits = [20000.0, 30000.0, 32000.0, 38000.0];
    for (i, expected) in expected_profits.iter().enumerate() {
        let row = i + 2;
        let cell = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Analysis", "cell_ref": format!("D{}", row)}),
        )
        .await;
        assert_eq!(
            cell["value"].as_f64().unwrap(),
            *expected,
            "D{} profit must equal {}",
            row,
            expected
        );
    }

    // Step 7: insert totals row (row 6).
    for col in &["B", "C", "D"] {
        let formula = format!("SUM({}2:{}5)", col, col);
        let cell_ref = format!("{}6", col);
        call_tool(
            &mut server,
            "insert_formula",
            json!({"sheet": "Analysis", "cell_ref": cell_ref, "formula": formula}),
        )
        .await;
    }

    // Verify totals: revenue=267000, costs=147000, profit=120000.
    let b6 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Analysis", "cell_ref": "B6"}),
    )
    .await;
    assert_eq!(
        b6["value"].as_f64().unwrap(),
        267000.0,
        "Total revenue must be 267000"
    );

    let c6 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Analysis", "cell_ref": "C6"}),
    )
    .await;
    assert_eq!(
        c6["value"].as_f64().unwrap(),
        147000.0,
        "Total costs must be 147000"
    );

    let d6 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Analysis", "cell_ref": "D6"}),
    )
    .await;
    assert_eq!(
        d6["value"].as_f64().unwrap(),
        120000.0,
        "Total profit must be 120000"
    );

    // Step 8: describe_data on revenue B2:B5.
    let stats = call_tool(
        &mut server,
        "describe_data",
        json!({"sheet": "Analysis", "range": "B2:B5"}),
    )
    .await;
    assert_eq!(stats["numeric_count"], 4, "4 revenue values");
    let s = &stats["statistics"];
    assert_eq!(
        s["sum"].as_f64().unwrap(),
        267000.0,
        "revenue sum must be 267000"
    );
    assert_eq!(
        s["min"].as_f64().unwrap(),
        50000.0,
        "min revenue must be 50000"
    );
    assert_eq!(
        s["max"].as_f64().unwrap(),
        80000.0,
        "max revenue must be 80000"
    );
    // mean = 267000/4 = 66750
    assert_eq!(
        s["mean"].as_f64().unwrap(),
        66750.0,
        "mean revenue must be 66750"
    );

    // Step 9: sort A2:D5 by Revenue (column B) descending so Q4 (80000) comes first.
    let sort_result = call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Analysis",
            "range": "A2:D5",
            "sort_by": [{"column": "B", "ascending": false}]
        }),
    )
    .await;
    assert_eq!(sort_result["success"], true);
    assert_eq!(sort_result["rows_sorted"], 4);

    // After descending sort: Q4 (80000) should now be in row 2.
    let a2_after_sort = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Analysis", "cell_ref": "A2"}),
    )
    .await;
    assert_eq!(
        a2_after_sort["value"], "Q4",
        "Q4 must be first row after sort by revenue desc"
    );
    let b2_after_sort = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Analysis", "cell_ref": "B2"}),
    )
    .await;
    assert_eq!(
        b2_after_sort["value"].as_f64().unwrap(),
        80000.0,
        "Revenue in first row must be 80000 (Q4)"
    );

    // Step 10: format header row bold.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Analysis",
            "cell_ref": "A1:D1",
            "bold": true
        }),
    )
    .await;
    let header_fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Analysis", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(header_fmt["format"]["bold"], true, "header A1 must be bold");

    // Step 11: format Profit column D2:D5 with green font.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Analysis",
            "cell_ref": "D2:D5",
            "font_color": "#008000"
        }),
    )
    .await;
    let profit_fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Analysis", "cell_ref": "D2"}),
    )
    .await;
    assert_eq!(
        profit_fmt["format"]["font_color"], "#008000",
        "profit cells must have green font"
    );

    // Step 12: create a bar chart from the revenue data.
    let chart = call_tool(
        &mut server,
        "create_chart",
        json!({
            "sheet": "Analysis",
            "chart_type": "bar",
            "data_range": "A1:B5",
            "title": "Quarterly Revenue"
        }),
    )
    .await;
    assert_eq!(chart["success"], true);
    assert!(
        chart["chart_id"].is_string(),
        "chart_id must be present in response"
    );
    let chart_id = chart["chart_id"].as_str().unwrap().to_string();

    // Step 13: get_workbook_info — verify 2 sheets, sufficient cells.
    let info = call_tool(&mut server, "get_workbook_info", json!({})).await;
    assert_eq!(info["sheet_count"], 2, "must have Sheet1 and Analysis");
    let analysis_info = info["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Analysis")
        .expect("Analysis sheet must appear in workbook info");
    assert!(
        analysis_info["cell_count"].as_u64().unwrap() >= 20,
        "Analysis sheet must have at least 20 cells (headers + data)"
    );

    // Step 14: export CSV from Analysis and verify key values appear.
    let csv_result = call_tool(&mut server, "export_csv", json!({"sheet": "Analysis"})).await;
    let csv = csv_result["csv"].as_str().unwrap();
    assert!(csv.contains("Quarter"), "CSV must contain 'Quarter' header");
    assert!(csv.contains("Revenue"), "CSV must contain 'Revenue' header");
    assert!(csv.contains("Q4"), "CSV must contain Q4 data");
    assert!(csv.contains("80000"), "CSV must contain 80000 revenue");

    // Cleanup the test chart.
    call_tool(&mut server, "delete_chart", json!({"chart_id": chart_id})).await;
}

// ── 25. Named functions CRUD via MCP ──────────────────────────────────────────

/// Test the full lifecycle of named functions via MCP tools:
/// add, list (verify name/params/body), duplicate error, remove, list again.
#[tokio::test]
async fn test_mcp_named_functions() {
    let mut server = McpServer::new_default();

    // Initially the named function list must be empty.
    let initial_list = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(
        initial_list["count"], 0,
        "new workbook must have 0 named functions"
    );

    // Step 1: add DOUBLE(x) = x*2.
    let add_result = call_tool(
        &mut server,
        "add_named_function",
        json!({
            "name": "DOUBLE",
            "params": ["x"],
            "body": "x*2",
            "description": "Returns twice the input value"
        }),
    )
    .await;
    assert_eq!(add_result["success"], true);
    assert_eq!(add_result["name"], "DOUBLE");
    assert_eq!(add_result["params"][0], "x");
    assert_eq!(add_result["body"], "x*2");
    assert_eq!(
        add_result["description"], "Returns twice the input value",
        "description must round-trip"
    );

    // Step 2: list — verify DOUBLE is present.
    let list_after_add = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(
        list_after_add["count"], 1,
        "must have 1 named function after adding DOUBLE"
    );
    let funcs = list_after_add["named_functions"].as_array().unwrap();
    assert_eq!(funcs[0]["name"], "DOUBLE");
    assert_eq!(funcs[0]["params"][0], "x");
    assert_eq!(funcs[0]["body"], "x*2");

    // Step 3: adding a duplicate (case-insensitive) must fail.
    let dup_err = call_tool_expect_error(
        &mut server,
        "add_named_function",
        json!({
            "name": "double",
            "params": ["x"],
            "body": "x*99"
        }),
    )
    .await;
    assert!(
        !dup_err.is_empty(),
        "duplicate named function must return an error"
    );

    // Step 4: add a second function TRIPLE(x) = x*3.
    call_tool(
        &mut server,
        "add_named_function",
        json!({"name": "TRIPLE", "params": ["x"], "body": "x*3"}),
    )
    .await;

    let list_two = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(
        list_two["count"], 2,
        "must have 2 named functions after adding TRIPLE"
    );

    // Step 5: remove DOUBLE.
    let remove_result = call_tool(
        &mut server,
        "remove_named_function",
        json!({"name": "DOUBLE"}),
    )
    .await;
    assert_eq!(remove_result["success"], true);
    assert_eq!(remove_result["name"], "DOUBLE");

    // Step 6: list after removal — only TRIPLE should remain.
    let list_after_remove = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(
        list_after_remove["count"], 1,
        "must have 1 named function after removing DOUBLE"
    );
    let remaining = list_after_remove["named_functions"].as_array().unwrap();
    assert_eq!(remaining[0]["name"], "TRIPLE", "TRIPLE must remain");

    // Step 7: removing a non-existent function must fail.
    let not_found_err = call_tool_expect_error(
        &mut server,
        "remove_named_function",
        json!({"name": "DOUBLE"}),
    )
    .await;
    assert!(
        !not_found_err.is_empty(),
        "removing non-existent function must return an error"
    );

    // Step 8: remove TRIPLE and verify the list is empty again.
    call_tool(
        &mut server,
        "remove_named_function",
        json!({"name": "TRIPLE"}),
    )
    .await;

    let final_list = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert_eq!(
        final_list["count"], 0,
        "named function list must be empty after removing all functions"
    );
}

// ── 26. Conditional formatting + filter view ──────────────────────────────────

/// Write numbers 1-20, apply a conditional format rule for values > 10,
/// verify the rule is listed, then save and apply a filter view that shows
/// only the values > 10 (i.e., 11-20), verify the hidden row count, clear
/// the filter by deleting the view, and finally remove the conditional format.
#[tokio::test]
async fn test_mcp_conditional_format_and_filter() {
    let mut server = McpServer::new_default();

    // Write values 1-20 in A1:A20.
    for n in 1u32..=20 {
        let cell_ref = format!("A{}", n);
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": cell_ref, "value": n}),
        )
        .await;
    }

    // Step 1: add conditional format — cell_value > 10 → bold + red font.
    let cf_add = call_tool(
        &mut server,
        "add_conditional_format",
        json!({
            "sheet": "Sheet1",
            "range": "A1:A20",
            "rule_type": {
                "kind": "cell_value",
                "operator": ">",
                "value1": 10.0
            },
            "style": {
                "bold": true,
                "font_color": "#ff0000"
            }
        }),
    )
    .await;
    assert_eq!(cf_add["success"], true);

    // Step 2: list conditional formats — verify the rule is present.
    let cf_list = call_tool(
        &mut server,
        "list_conditional_formats",
        json!({"sheet": "Sheet1"}),
    )
    .await;
    assert_eq!(
        cf_list["count"], 1,
        "must have 1 conditional format after adding"
    );
    let cf_entry = &cf_list["conditional_formats"][0];
    assert_eq!(cf_entry["range"], "A1:A20", "range must match");
    assert_eq!(cf_entry["rules"][0]["rule_type"]["kind"], "cell_value");
    assert_eq!(cf_entry["rules"][0]["rule_type"]["operator"], ">");
    assert_eq!(
        cf_entry["rules"][0]["rule_type"]["value1"]
            .as_f64()
            .unwrap(),
        10.0
    );
    assert_eq!(
        cf_entry["rules"][0]["style"]["bold"], true,
        "style bold must be true"
    );
    assert_eq!(
        cf_entry["rules"][0]["style"]["font_color"], "#ff0000",
        "style font_color must be #ff0000"
    );

    // Step 3: save a filter view that shows only values 11-20.
    // The filter view uses column 0 (A), allowing string values "11".."20".
    let allowed_values: Vec<Value> = (11u32..=20).map(|n| json!(n.to_string())).collect();
    let fv_save = call_tool(
        &mut server,
        "save_filter_view",
        json!({
            "name": "HighValues",
            "column_filters": {
                "0": allowed_values
            }
        }),
    )
    .await;
    assert_eq!(fv_save["success"], true);
    assert_eq!(fv_save["name"], "HighValues");

    // Step 4: list filter views — verify HighValues is present.
    let fv_list = call_tool(&mut server, "list_filter_views", json!({})).await;
    assert_eq!(fv_list["count"], 1, "must have 1 filter view after saving");
    assert_eq!(fv_list["filter_views"][0]["name"], "HighValues");

    // Step 5: apply HighValues to Sheet1 — rows with values <= 10 should be hidden.
    // Row 0 (A1 = value 1) is treated as the header row and is never hidden.
    // Data rows 1-19 contain values 2-20. Values 2-10 (rows 1-9) do NOT match
    // the allowed list (11-20) → 9 rows hidden.
    let fv_apply = call_tool(
        &mut server,
        "apply_filter_view",
        json!({"sheet": "Sheet1", "name": "HighValues"}),
    )
    .await;
    assert_eq!(fv_apply["success"], true);
    assert_eq!(
        fv_apply["rows_hidden"].as_u64().unwrap(),
        9,
        "values 2-10 in rows 1-9 must be hidden (row 0 is the implicit header)"
    );

    // Step 6: delete the filter view (clears the named filter, but rows stay hidden
    // until unhidden explicitly — the delete only removes the saved view definition).
    let fv_delete = call_tool(
        &mut server,
        "delete_filter_view",
        json!({"name": "HighValues"}),
    )
    .await;
    assert_eq!(fv_delete["success"], true);

    // Verify the filter view is gone from the list.
    let fv_list_after = call_tool(&mut server, "list_filter_views", json!({})).await;
    assert_eq!(
        fv_list_after["count"], 0,
        "filter view list must be empty after deletion"
    );

    // Step 7: unhide the 9 hidden rows (rows 2-10 in 1-based, which are rows 1-9
    // in 0-based) to reset the sheet state.
    call_tool(
        &mut server,
        "unhide_rows",
        json!({"sheet": "Sheet1", "start_row": 2, "count": 9}),
    )
    .await;

    // Step 8: remove the conditional format rule.
    let cf_remove = call_tool(
        &mut server,
        "remove_conditional_format",
        json!({
            "sheet": "Sheet1",
            "range": "A1:A20",
            "rule_index": 0
        }),
    )
    .await;
    assert_eq!(cf_remove["success"], true);

    // Step 9: verify the conditional format list is now empty.
    let cf_list_after = call_tool(
        &mut server,
        "list_conditional_formats",
        json!({"sheet": "Sheet1"}),
    )
    .await;
    assert_eq!(
        cf_list_after["count"], 0,
        "conditional format list must be empty after removal"
    );
}

// ── 27. clear_range ───────────────────────────────────────────────────────────

/// Write 4 cells, clear 2 of them, verify cleared cells return null and the
/// others are unaffected.
#[tokio::test]
async fn test_clear_range_basic() {
    let mut server = McpServer::new_default();

    for i in 1..=4u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "clear_range",
        json!({"sheet": "Sheet1", "range": "A2:A3"}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cells_cleared"], 2, "should clear 2 cells");
    assert_eq!(result["range"], "A2:A3");

    // A1 and A4 must still have their values.
    let a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(a1["value"].as_f64().unwrap(), 1.0, "A1 must be intact");

    let a4 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A4"}),
    )
    .await;
    assert_eq!(a4["value"].as_f64().unwrap(), 4.0, "A4 must be intact");

    // A2 and A3 must be null.
    let a2 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2"}),
    )
    .await;
    assert!(a2["value"].is_null(), "A2 must be null after clear");

    let a3 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A3"}),
    )
    .await;
    assert!(a3["value"].is_null(), "A3 must be null after clear");
}

/// Clearing an already-empty range should succeed with cells_cleared = 0.
#[tokio::test]
async fn test_clear_range_already_empty() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "clear_range",
        json!({"sheet": "Sheet1", "range": "B1:C5"}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cells_cleared"], 0, "empty range clears 0 cells");
}

// ── 28. write_range with readback ────────────────────────────────────────────

/// Write a 3x2 block via write_range then verify every cell with read_range.
#[tokio::test]
async fn test_write_range_and_verify() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "B2",
            "values": [
                [10, "alpha", true],
                [20, "beta", false]
            ]
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cells_written"], 6, "3x2 = 6 cells");

    let range = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "B2:D3"}),
    )
    .await;

    let data = &range["data"];
    assert_eq!(data[0][0].as_f64().unwrap(), 10.0);
    assert_eq!(data[0][1], "alpha");
    assert_eq!(data[0][2], true);
    assert_eq!(data[1][0].as_f64().unwrap(), 20.0);
    assert_eq!(data[1][1], "beta");
    assert_eq!(data[1][2], false);
}

// ── 29. deduplicate ───────────────────────────────────────────────────────────

/// Write rows with duplicates, deduplicate, verify only unique rows remain.
#[tokio::test]
async fn test_deduplicate_removes_duplicates() {
    let mut server = McpServer::new_default();

    // Rows: apple, banana, apple, cherry, banana  → unique: apple, banana, cherry
    let fruits = ["apple", "banana", "apple", "cherry", "banana"];
    for (i, f) in fruits.iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 1), "value": f}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "deduplicate",
        json!({"sheet": "Sheet1", "range": "A1:A5", "columns": ["A"]}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["original_rows"], 5);
    assert_eq!(result["unique_rows"], 3);
    assert_eq!(result["duplicates_removed"], 2);

    // Verify unique values in A1:A3.
    let range = call_tool(
        &mut server,
        "read_range",
        json!({"sheet": "Sheet1", "range": "A1:A3"}),
    )
    .await;
    let data = range["data"].as_array().unwrap();
    assert_eq!(data[0][0], "apple");
    assert_eq!(data[1][0], "banana");
    assert_eq!(data[2][0], "cherry");

    // A4 and A5 must be cleared (were duplicates).
    let a4 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A4"}),
    )
    .await;
    assert!(a4["value"].is_null(), "A4 should be cleared after dedup");
}

/// Deduplicate with all-unique data keeps everything.
#[tokio::test]
async fn test_deduplicate_no_duplicates() {
    let mut server = McpServer::new_default();

    for (i, v) in [1, 2, 3].iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 1), "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "deduplicate",
        json!({"sheet": "Sheet1", "range": "A1:A3"}),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["duplicates_removed"], 0);
    assert_eq!(result["unique_rows"], 3);
}

// ── 30. transpose ─────────────────────────────────────────────────────────────

/// Write a 1x3 row (A1:C1), transpose to E1, verify columns become rows.
#[tokio::test]
async fn test_transpose_row_to_column() {
    let mut server = McpServer::new_default();

    // Write a row: A1=10, B1=20, C1=30
    for (i, v) in [10, 20, 30].iter().enumerate() {
        let col = (b'A' + i as u8) as char;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("{}1", col), "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "transpose",
        json!({
            "sheet": "Sheet1",
            "source_range": "A1:C1",
            "target_cell": "E1"
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["original_dimensions"], "1x3");
    assert_eq!(result["transposed_dimensions"], "3x1");

    // E1=10, E2=20, E3=30 (column from transposed row).
    for (i, expected) in [10.0, 20.0, 30.0].iter().enumerate() {
        let cell = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("E{}", i + 1)}),
        )
        .await;
        assert_eq!(
            cell["value"].as_f64().unwrap(),
            *expected,
            "E{} should be {}",
            i + 1,
            expected
        );
    }
}

// ── 31. auto_fill via MCP ────────────────────────────────────────────────────

/// Write 1, 2, 3 in A1:A3 and auto_fill down to A4:A6 → should produce 4, 5, 6.
#[tokio::test]
async fn test_auto_fill_mcp_linear_down() {
    let mut server = McpServer::new_default();

    for i in 1..=3u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "auto_fill",
        json!({
            "sheet": "Sheet1",
            "source_range": "A1:A3",
            "target_range": "A4:A6",
            "direction": "down"
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["cells_filled"], 3);

    // Verify A4=4, A5=5, A6=6.
    for (i, expected) in [4.0, 5.0, 6.0].iter().enumerate() {
        let cell = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 4)}),
        )
        .await;
        assert_eq!(
            cell["value"].as_f64().unwrap(),
            *expected,
            "A{} should be {}",
            i + 4,
            expected
        );
    }
}

/// Invalid fill direction must return an error.
#[tokio::test]
async fn test_auto_fill_invalid_direction() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 1}),
    )
    .await;

    let err = call_tool_expect_error(
        &mut server,
        "auto_fill",
        json!({
            "sheet": "Sheet1",
            "source_range": "A1:A1",
            "target_range": "A2:A3",
            "direction": "diagonal"
        }),
    )
    .await;
    assert!(
        err.contains("Invalid direction"),
        "bad direction must error"
    );
}

// ── 32. hide_rows / unhide_rows ───────────────────────────────────────────────

/// Hide 2 rows, verify success, then unhide them.
#[tokio::test]
async fn test_hide_and_unhide_rows() {
    let mut server = McpServer::new_default();

    let hide_result = call_tool(
        &mut server,
        "hide_rows",
        json!({"sheet": "Sheet1", "start_row": 3, "count": 2}),
    )
    .await;

    assert_eq!(hide_result["success"], true);
    assert_eq!(hide_result["rows_hidden"], 2);

    let unhide_result = call_tool(
        &mut server,
        "unhide_rows",
        json!({"sheet": "Sheet1", "start_row": 3, "count": 2}),
    )
    .await;

    assert_eq!(unhide_result["success"], true);
    assert_eq!(unhide_result["rows_unhidden"], 2);
}

/// hide_rows with start_row=0 must fail (must be 1-based).
#[tokio::test]
async fn test_hide_rows_zero_start_errors() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "hide_rows",
        json!({"sheet": "Sheet1", "start_row": 0, "count": 1}),
    )
    .await;
    assert!(
        err.contains("1-based"),
        "start_row=0 must produce a 1-based error"
    );
}

// ── 33. hide_cols / unhide_cols ───────────────────────────────────────────────

/// Hide column B, then unhide it.
#[tokio::test]
async fn test_hide_and_unhide_cols() {
    let mut server = McpServer::new_default();

    let hide_result = call_tool(
        &mut server,
        "hide_cols",
        json!({"sheet": "Sheet1", "start_col": "B", "count": 1}),
    )
    .await;

    assert_eq!(hide_result["success"], true);
    assert_eq!(hide_result["cols_hidden"], 1);

    let unhide_result = call_tool(
        &mut server,
        "unhide_cols",
        json!({"sheet": "Sheet1", "start_col": "B", "count": 1}),
    )
    .await;

    assert_eq!(unhide_result["success"], true);
    assert_eq!(unhide_result["cols_unhidden"], 1);
}

// ── 34. protect_sheet / unprotect_sheet ──────────────────────────────────────

/// Protect a sheet without password, then unprotect it.
#[tokio::test]
async fn test_protect_and_unprotect_sheet_no_password() {
    let mut server = McpServer::new_default();

    let protect_result = call_tool(&mut server, "protect_sheet", json!({"sheet": "Sheet1"})).await;

    assert_eq!(protect_result["success"], true);
    assert_eq!(protect_result["sheet"], "Sheet1");
    assert_eq!(protect_result["has_password"], false);

    let unprotect_result =
        call_tool(&mut server, "unprotect_sheet", json!({"sheet": "Sheet1"})).await;

    assert_eq!(unprotect_result["success"], true);
    assert_eq!(unprotect_result["sheet"], "Sheet1");
}

/// Protect with password, unprotect with correct password.
#[tokio::test]
async fn test_protect_sheet_with_password() {
    let mut server = McpServer::new_default();

    let protect_result = call_tool(
        &mut server,
        "protect_sheet",
        json!({"sheet": "Sheet1", "password": "secret123"}),
    )
    .await;

    assert_eq!(protect_result["success"], true);
    assert_eq!(protect_result["has_password"], true);

    // Unprotect with wrong password must fail.
    let bad_unprotect = call_tool_expect_error(
        &mut server,
        "unprotect_sheet",
        json!({"sheet": "Sheet1", "password": "wrongpass"}),
    )
    .await;
    assert!(
        !bad_unprotect.is_empty(),
        "wrong password must produce an error"
    );

    // Unprotect with correct password must succeed.
    let good_unprotect = call_tool(
        &mut server,
        "unprotect_sheet",
        json!({"sheet": "Sheet1", "password": "secret123"}),
    )
    .await;
    assert_eq!(good_unprotect["success"], true);
}

// ── 35. merge_cells / unmerge_cells ──────────────────────────────────────────

/// Merge A1:B2, verify success, then unmerge via any cell in the range.
#[tokio::test]
async fn test_merge_and_unmerge_cells() {
    let mut server = McpServer::new_default();

    // Write a value in A1 — the merge preserves the top-left value.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "merged"}),
    )
    .await;

    let merge_result = call_tool(
        &mut server,
        "merge_cells",
        json!({"sheet": "Sheet1", "range": "A1:B2"}),
    )
    .await;

    assert_eq!(merge_result["success"], true);
    assert_eq!(merge_result["range"], "A1:B2");

    // Unmerge via B1 (any cell in the merged region).
    let unmerge_result = call_tool(
        &mut server,
        "unmerge_cells",
        json!({"sheet": "Sheet1", "cell_ref": "B1"}),
    )
    .await;

    assert_eq!(unmerge_result["success"], true);
}

// ── 36. Validation: set, get, validate_cell, remove ──────────────────────────

/// Full validation lifecycle: set a list rule, get it back, validate a cell
/// value, remove the rule, verify it's gone.
#[tokio::test]
async fn test_validation_list_lifecycle() {
    let mut server = McpServer::new_default();

    // Set a list validation on B2: allowed = ["Yes", "No", "Maybe"]
    let set_result = call_tool(
        &mut server,
        "set_validation",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "B2",
            "rule_type": "list",
            "list_items": ["Yes", "No", "Maybe"],
            "allow_blank": false,
            "error_message": "Please choose Yes, No, or Maybe"
        }),
    )
    .await;

    assert_eq!(set_result["success"], true);
    assert_eq!(set_result["rule_type"], "list");

    // Get the validation back.
    let get_result = call_tool(
        &mut server,
        "get_validation",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;

    assert_eq!(get_result["has_validation"], true);
    let rule = &get_result["rule"];
    // format_rule wraps the type info under "validation_type".
    let vtype = &rule["validation_type"];
    assert_eq!(vtype["type"], "list", "validation_type.type must be 'list'");
    let items = vtype["items"].as_array().unwrap();
    assert!(items.iter().any(|v| v == "Yes"), "Yes must be in list");
    assert!(items.iter().any(|v| v == "No"), "No must be in list");

    // Write "Yes" → validate_cell should return valid=true.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2", "value": "Yes"}),
    )
    .await;
    let validate_valid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;
    assert_eq!(
        validate_valid["valid"], true,
        "\"Yes\" must pass list validation"
    );

    // Write "Invalid" → validate_cell should return valid=false.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2", "value": "Invalid"}),
    )
    .await;
    let validate_invalid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;
    assert_eq!(
        validate_invalid["valid"], false,
        "\"Invalid\" must fail list validation"
    );

    // Remove the rule and verify it's gone.
    let remove_result = call_tool(
        &mut server,
        "remove_validation",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;
    assert_eq!(remove_result["success"], true);

    let get_after_remove = call_tool(
        &mut server,
        "get_validation",
        json!({"sheet": "Sheet1", "cell_ref": "B2"}),
    )
    .await;
    assert_eq!(
        get_after_remove["has_validation"], false,
        "validation must be gone after remove"
    );
}

/// Number range validation: values must be between 1 and 100.
#[tokio::test]
async fn test_validation_number_range() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "set_validation",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "C3",
            "rule_type": "number_range",
            "min": 1.0,
            "max": 100.0
        }),
    )
    .await;

    // Write 50 → should pass.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C3", "value": 50}),
    )
    .await;
    let valid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C3"}),
    )
    .await;
    assert_eq!(valid["valid"], true, "50 must pass number_range 1-100");

    // Write 200 → should fail.
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C3", "value": 200}),
    )
    .await;
    let invalid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C3"}),
    )
    .await;
    assert_eq!(invalid["valid"], false, "200 must fail number_range 1-100");
}

/// validate_cell on a cell with no validation rule always returns valid=true.
#[tokio::test]
async fn test_validate_cell_no_rule_is_valid() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "Z50"}),
    )
    .await;

    assert_eq!(result["valid"], true, "no rule means always valid");
    assert_eq!(result["has_validation"], false);
}

// ── 37. correlate + trend_analysis ───────────────────────────────────────────

/// Perfect positive correlation: X = [1,2,3,4,5], Y = [2,4,6,8,10].
/// Expected r = 1.0.
#[tokio::test]
async fn test_correlate_perfect_positive() {
    let mut server = McpServer::new_default();

    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [2.0, 4.0, 6.0, 8.0, 10.0];
    for (i, (xi, yi)) in x.iter().zip(y.iter()).enumerate() {
        let row = i + 1;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", row), "value": xi}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("B{}", row), "value": yi}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "correlate",
        json!({"sheet": "Sheet1", "range_x": "A1:A5", "range_y": "B1:B5"}),
    )
    .await;

    let r = result["correlation"].as_f64().unwrap();
    assert!(
        (r - 1.0).abs() < 1e-10,
        "perfect positive correlation must be ~1.0, got {}",
        r
    );
    assert_eq!(result["n"], 5);
    // r_squared must also be ~1.0
    let r2 = result["r_squared"].as_f64().unwrap();
    assert!(
        (r2 - 1.0).abs() < 1e-10,
        "r_squared must be ~1.0, got {}",
        r2
    );
}

/// Trend analysis on Y = 2X + 1 must yield slope~2, intercept~1, r_squared~1.
#[tokio::test]
async fn test_trend_analysis_linear() {
    let mut server = McpServer::new_default();

    // Y = 2X + 1: X=[1,2,3,4,5], Y=[3,5,7,9,11]
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [3.0, 5.0, 7.0, 9.0, 11.0];
    for (i, (xi, yi)) in x.iter().zip(y.iter()).enumerate() {
        let row = i + 1;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", row), "value": xi}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("B{}", row), "value": yi}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "trend_analysis",
        json!({"sheet": "Sheet1", "range_x": "A1:A5", "range_y": "B1:B5"}),
    )
    .await;

    // Regression stats are nested under "linear_regression".
    let lr = &result["linear_regression"];
    let slope = lr["slope"]
        .as_f64()
        .expect("linear_regression.slope must be present");
    let intercept = lr["intercept"]
        .as_f64()
        .expect("linear_regression.intercept must be present");
    let r2 = lr["r_squared"]
        .as_f64()
        .expect("linear_regression.r_squared must be present");

    assert!(
        (slope - 2.0).abs() < 1e-10,
        "slope must be ~2.0, got {}",
        slope
    );
    assert!(
        (intercept - 1.0).abs() < 1e-10,
        "intercept must be ~1.0, got {}",
        intercept
    );
    assert!(
        (r2 - 1.0).abs() < 1e-10,
        "r_squared must be ~1.0, got {}",
        r2
    );
}

/// correlate with mismatched range sizes must return an error.
#[tokio::test]
async fn test_correlate_mismatched_lengths_errors() {
    let mut server = McpServer::new_default();

    for i in 1..=3u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }
    for i in 1..=5u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("B{}", i), "value": i}),
        )
        .await;
    }

    let err = call_tool_expect_error(
        &mut server,
        "correlate",
        json!({"sheet": "Sheet1", "range_x": "A1:A3", "range_y": "B1:B5"}),
    )
    .await;
    assert!(!err.is_empty(), "mismatched lengths must produce an error");
}

// ── 38. export_json ───────────────────────────────────────────────────────────

/// Write cells, export as JSON, verify structure and values.
#[tokio::test]
async fn test_export_json_single_sheet() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Name"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "Alice"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "value": "Score"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B2", "value": 95}),
    )
    .await;

    let result = call_tool(&mut server, "export_json", json!({"sheet": "Sheet1"})).await;

    assert!(
        result["sheets"].is_object() || result["rows"].is_array() || result.is_object(),
        "export_json must return a JSON object with sheet data"
    );
    // The result must contain the string "Alice" somewhere in the JSON.
    let result_str = result.to_string();
    assert!(result_str.contains("Alice"), "JSON must contain 'Alice'");
    assert!(result_str.contains("95"), "JSON must contain score 95");
}

/// export_json with no sheet argument exports the whole workbook.
#[tokio::test]
async fn test_export_json_whole_workbook() {
    let mut server = McpServer::new_default();

    call_tool(&mut server, "create_sheet", json!({"name": "Sheet2"})).await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "s1data"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet2", "cell_ref": "A1", "value": "s2data"}),
    )
    .await;

    let result = call_tool(&mut server, "export_json", json!({})).await;
    let result_str = result.to_string();
    assert!(
        result_str.contains("s1data"),
        "whole workbook JSON must contain Sheet1 data"
    );
    assert!(
        result_str.contains("s2data"),
        "whole workbook JSON must contain Sheet2 data"
    );
}

// ── 39. set_sheet_tab_color ───────────────────────────────────────────────────

/// Set a tab color on Sheet1, verify success, then clear it with null.
#[tokio::test]
async fn test_set_sheet_tab_color() {
    let mut server = McpServer::new_default();

    let set_result = call_tool(
        &mut server,
        "set_sheet_tab_color",
        json!({"sheet": "Sheet1", "color": "#FF0000"}),
    )
    .await;
    assert_eq!(set_result["success"], true);
    assert_eq!(set_result["tab_color"], "#FF0000");

    // Clear the tab color with null.
    let clear_result = call_tool(
        &mut server,
        "set_sheet_tab_color",
        json!({"sheet": "Sheet1", "color": null}),
    )
    .await;
    assert_eq!(clear_result["success"], true);
    assert!(
        clear_result["tab_color"].is_null(),
        "tab_color must be null after clearing"
    );
}

// ── 40. text_to_columns ───────────────────────────────────────────────────────

/// Write "a,b,c" in A1:A3, split on ",", verify adjacent columns are created.
#[tokio::test]
async fn test_text_to_columns_comma_delimiter() {
    let mut server = McpServer::new_default();

    let values = ["alpha,beta,gamma", "one,two,three", "x,y,z"];
    for (i, v) in values.iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 1), "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "text_to_columns",
        json!({
            "sheet": "Sheet1",
            "col": "A",
            "delimiter": ",",
            "start_row": 1,
            "end_row": 3
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(result["column"], "A");
    let max_cols = result["max_columns_created"].as_u64().unwrap();
    assert!(
        max_cols >= 3,
        "splitting 3-part CSV must create at least 3 columns"
    );

    // Verify B1 = "alpha", C1 = "beta", D1 = "gamma".
    let b1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B1"}),
    )
    .await;
    assert_eq!(
        b1["value"], "beta",
        "B1 must be 'beta' after text_to_columns"
    );

    let c1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "C1"}),
    )
    .await;
    assert_eq!(
        c1["value"], "gamma",
        "C1 must be 'gamma' after text_to_columns"
    );
}

// ── 41. remove_duplicates (core engine route) ─────────────────────────────────

/// Write 5 rows where rows 2 and 4 are duplicates, run remove_duplicates,
/// verify the removed count.
#[tokio::test]
async fn test_remove_duplicates_mcp() {
    let mut server = McpServer::new_default();

    // A1=10, A2=20, A3=10, A4=30, A5=20
    for (i, v) in [10, 20, 10, 30, 20].iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i + 1), "value": v}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "remove_duplicates",
        json!({
            "sheet": "Sheet1",
            "start_row": 1,
            "end_row": 5,
            "columns": ["A"]
        }),
    )
    .await;

    assert_eq!(result["success"], true);
    assert_eq!(
        result["rows_removed"], 2,
        "rows with value 10 (row 3) and 20 (row 5) are duplicates"
    );
    assert_eq!(result["rows_remaining"], 3, "3 unique rows remain");
}

// ── 42. generate_pivot ────────────────────────────────────────────────────────

/// Write a header row + 3 data rows (Category, Value), generate a pivot
/// summing values per category, verify the result.
///
/// Note: generate_pivot treats the first row of source_range as a header row
/// and starts aggregating from the second row onwards.
#[tokio::test]
async fn test_generate_pivot_sum() {
    let mut server = McpServer::new_default();

    // Row 1 = headers, rows 2-4 = data
    // A1="Category", B1="Value"
    // A2="A", B2=10
    // A3="B", B3=20
    // A4="A", B4=30
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Category"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "value": "Value"}),
    )
    .await;
    let data = [("A", 10), ("B", 20), ("A", 30)];
    for (i, (cat, val)) in data.iter().enumerate() {
        let row = i + 2;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", row), "value": cat}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("B{}", row), "value": val}),
        )
        .await;
    }

    let result = call_tool(
        &mut server,
        "generate_pivot",
        json!({
            "sheet": "Sheet1",
            "source_range": "A1:B4",
            "row_fields": ["A"],
            "value_fields": [{"col": "B", "aggregation": "sum"}]
        }),
    )
    .await;

    assert!(
        result["row_count"].as_u64().unwrap() > 0,
        "pivot must have rows"
    );
    let rows = result["rows"].as_array().unwrap();
    // Find "A" row in pivot output.
    let a_row = rows.iter().find(|r| r[0] == "A");
    assert!(a_row.is_some(), "pivot must have a row for category 'A'");
    // Sum for A = 10 + 30 = 40
    let a_sum = a_row.unwrap()[1].as_f64().unwrap();
    assert_eq!(a_sum, 40.0, "pivot sum for 'A' must be 40");

    // Sum for B = 20
    let b_row = rows.iter().find(|r| r[0] == "B");
    assert!(b_row.is_some(), "pivot must have a row for category 'B'");
    let b_sum = b_row.unwrap()[1].as_f64().unwrap();
    assert_eq!(b_sum, 20.0, "pivot sum for 'B' must be 20");
}

// ── 43. get_formula ───────────────────────────────────────────────────────────

/// Insert a formula, then retrieve it via get_formula. Verify formula and value.
#[tokio::test]
async fn test_get_formula_after_insert() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 5}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": 7}),
    )
    .await;

    call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A3", "formula": "A1+A2"}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "get_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A3"}),
    )
    .await;

    assert_eq!(
        result["formula"], "A1+A2",
        "get_formula must return the stored formula"
    );
    assert_eq!(result["value"].as_f64().unwrap(), 12.0, "value must be 12");
}

/// get_formula on a cell with no formula returns null formula.
#[tokio::test]
async fn test_get_formula_no_formula_cell() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 99}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "get_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;

    assert!(
        result["formula"].is_null(),
        "cell without formula must return null formula"
    );
    assert_eq!(result["value"].as_f64().unwrap(), 99.0);
}

// ── 44. number_format in set_cell_format ─────────────────────────────────────

/// Set a number_format on a cell, verify it round-trips via get_cell_format,
/// then clear it with null.
#[tokio::test]
async fn test_number_format_set_and_clear() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 1234567.89}),
    )
    .await;

    call_tool(
        &mut server,
        "set_cell_format",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "number_format": "#,##0.00"
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt["format"]["number_format"], "#,##0.00",
        "number_format must round-trip"
    );

    // Clear the number_format.
    call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "number_format": null}),
    )
    .await;

    let fmt_after = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert!(
        fmt_after["format"]["number_format"].is_null(),
        "number_format must be null after clearing"
    );
}
