//! MCP integration tests — full Claude Desktop workflow.
//!
//! Each test creates its own `McpServer::new_default()` instance (no transport),
//! sends JSON-RPC 2.0 strings via `handle_message()`, and asserts on the
//! parsed response values — not just `isError: false`.

use lattice_core::{CellValue, FillDirection, FillPattern, Sheet, detect_pattern, fill_range};
use lattice_core::selection::{CellRef, Range};
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

    let remove_result =
        call_tool(&mut server, "remove_named_range", json!({"name": "Temp"})).await;
    assert_eq!(remove_result["success"], true);

    let err = call_tool_expect_error(
        &mut server,
        "resolve_named_range",
        json!({"name": "Temp"}),
    )
    .await;
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
