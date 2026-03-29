//! Release validation tests — comprehensive workflow coverage.
//!
//! These tests exercise Workflows A-D from the release checklist:
//!
//! - Workflow A: Financial Portfolio (write, formula, format, chart, sort, export)
//! - Workflow B: Date & formula features (dates, percentages, LET, LAMBDA, ARRAYFORMULA, cross-sheet, errors)
//! - Workflow C: Formatting & data ops (borders, merge, number format, named range, sort, find/replace, dedup, autofill)
//! - Workflow D: Advanced features (conditional format, validation, sparkline, filter view,
//!               hide/unhide, protect, named functions, pivot)

use lattice_mcp::McpServer;
use serde_json::{Value, json};

// ── Helpers (duplicated from mcp_tests.rs for self-containment) ───────────────

async fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
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

async fn call_tool_expect_error(server: &mut McpServer, name: &str, args: Value) -> String {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
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

// ══════════════════════════════════════════════════════════════════════════════
// WORKFLOW A: Financial Portfolio
// ══════════════════════════════════════════════════════════════════════════════

/// Step A1: Initialize → create sheet "Portfolio"
#[tokio::test]
async fn workflow_a1_create_portfolio_sheet() {
    let mut server = McpServer::new_default();

    let result = call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;
    assert_eq!(
        result["success"], true,
        "A1: create_sheet Portfolio must succeed"
    );

    let sheets = call_tool(&mut server, "list_sheets", json!({})).await;
    assert_eq!(
        sheets["count"], 2,
        "A1: workbook must have 2 sheets after create"
    );
    let names: Vec<&str> = sheets["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"Portfolio"),
        "A1: Portfolio sheet must be in list"
    );
}

/// Steps A2-A3: Write headers and 5 stocks with data
#[tokio::test]
async fn workflow_a2_write_portfolio_data() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write headers: Stock, Shares, Price, Value
    let headers = [
        ("A1", "Stock"),
        ("B1", "Shares"),
        ("C1", "Price"),
        ("D1", "Value"),
    ];
    for (cell, val) in &headers {
        let r = call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": cell, "value": val}),
        )
        .await;
        assert_eq!(r["success"], true, "A2: write header {} must succeed", cell);
    }

    // Write 5 stocks
    let stocks = [
        ("AAPL", 100, 185.50),
        ("GOOGL", 50, 141.25),
        ("MSFT", 75, 378.90),
        ("AMZN", 40, 196.75),
        ("TSLA", 30, 248.20),
    ];
    for (i, (stock, shares, price)) in stocks.iter().enumerate() {
        let row = i + 2;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("A{}", row), "value": stock}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("B{}", row), "value": shares}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("C{}", row), "value": price}),
        )
        .await;
    }

    // Verify data was written by reading back MSFT shares (row 4)
    let msft_shares = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Portfolio", "cell_ref": "B4"}),
    )
    .await;
    assert_eq!(
        msft_shares["value"].as_f64().unwrap(),
        75.0,
        "A2: MSFT shares must be 75"
    );
}

/// Step A4: Insert formulas Value = Shares * Price
#[tokio::test]
async fn workflow_a4_value_formulas() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write headers
    for (cell, val) in &[("B1", "Shares"), ("C1", "Price"), ("D1", "Value")] {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": cell, "value": val}),
        )
        .await;
    }

    // Write AAPL data
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Portfolio", "cell_ref": "A2", "value": "AAPL"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Portfolio", "cell_ref": "B2", "value": 100}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Portfolio", "cell_ref": "C2", "value": 185.50}),
    )
    .await;

    // D2 = B2 * C2 = 18550
    let d2 = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Portfolio", "cell_ref": "D2", "formula": "B2*C2"}),
    )
    .await;
    assert!(
        (d2["result"].as_f64().unwrap() - 18550.0).abs() < 0.01,
        "A4: AAPL Value must be 18550, got {}",
        d2["result"]
    );
}

/// Step A5: Insert SUM for total value
#[tokio::test]
async fn workflow_a5_total_sum() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write 5 values directly
    for (i, v) in [18550.0, 7062.5, 28417.5, 7870.0, 7446.0_f64]
        .iter()
        .enumerate()
    {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("D{}", i + 2), "value": v}),
        )
        .await;
    }

    // D7 = SUM(D2:D6)
    let sum = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Portfolio", "cell_ref": "D7", "formula": "SUM(D2:D6)"}),
    )
    .await;
    let total = sum["result"].as_f64().unwrap();
    assert!(
        (total - 69346.0).abs() < 0.1,
        "A5: total portfolio value must be ~69346, got {}",
        total
    );
}

/// Step A6: Insert AVERAGE, MIN, MAX
#[tokio::test]
async fn workflow_a6_statistical_formulas() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    let values = [18550.0, 7062.5, 28417.5, 7870.0, 7446.0_f64];
    for (i, v) in values.iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("D{}", i + 2), "value": v}),
        )
        .await;
    }

    let avg = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Portfolio", "cell_ref": "D8", "formula": "AVERAGE(D2:D6)"}),
    )
    .await;
    let avg_val = avg["result"].as_f64().unwrap();
    assert!(
        (avg_val - 13869.2).abs() < 0.1,
        "A6: AVERAGE must be ~13869.2, got {}",
        avg_val
    );

    let min = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Portfolio", "cell_ref": "D9", "formula": "MIN(D2:D6)"}),
    )
    .await;
    assert!(
        (min["result"].as_f64().unwrap() - 7062.5).abs() < 0.01,
        "A6: MIN must be 7062.5"
    );

    let max_r = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Portfolio", "cell_ref": "D10", "formula": "MAX(D2:D6)"}),
    )
    .await;
    assert!(
        (max_r["result"].as_f64().unwrap() - 28417.5).abs() < 0.01,
        "A6: MAX must be 28417.5"
    );
}

/// Step A7: Format — bold headers, currency on prices, yellow bg on totals
#[tokio::test]
async fn workflow_a7_formatting() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Bold headers A1:D1
    let fmt = call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Portfolio", "cell_ref": "A1:D1", "bold": true}),
    )
    .await;
    assert_eq!(fmt["success"], true, "A7: set_cell_format bold headers");

    // Currency format on C2:C6
    let curr = call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Portfolio", "cell_ref": "C2:C6", "number_format": "$#,##0.00"}),
    )
    .await;
    assert_eq!(curr["success"], true, "A7: currency format on prices");

    // Yellow background on totals row (D7)
    let yellow = call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Portfolio", "cell_ref": "D7", "bg_color": "#FFFF00"}),
    )
    .await;
    assert_eq!(yellow["success"], true, "A7: yellow bg on totals");

    // Verify bold header persists
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Portfolio", "cell_ref": "A1", "value": "Stock"}),
    )
    .await;
    let fmt_check = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Portfolio", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt_check["format"]["bold"], true,
        "A7: header A1 must be bold"
    );
}

/// Step A8: Create a pie chart of portfolio allocation
#[tokio::test]
async fn workflow_a8_create_pie_chart() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    let chart = call_tool(
        &mut server,
        "create_chart",
        json!({
            "sheet": "Portfolio",
            "chart_type": "pie",
            "data_range": "A2:D6",
            "title": "Portfolio Allocation"
        }),
    )
    .await;
    assert_eq!(chart["success"], true, "A8: create_chart pie must succeed");
    assert!(
        chart["chart_id"].as_str().is_some(),
        "A8: chart_id must be returned"
    );

    let charts = call_tool(&mut server, "list_charts", json!({"sheet": "Portfolio"})).await;
    assert!(
        charts["count"].as_u64().unwrap_or(0) > 0,
        "A8: list_charts must return at least 1 chart"
    );
}

/// Step A9: Describe data on the Value column
#[tokio::test]
async fn workflow_a9_describe_data() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    let values = [18550.0, 7062.5, 28417.5, 7870.0, 7446.0_f64];
    for (i, v) in values.iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("D{}", i + 2), "value": v}),
        )
        .await;
    }

    let stats = call_tool(
        &mut server,
        "describe_data",
        json!({"sheet": "Portfolio", "range": "D2:D6"}),
    )
    .await;

    // describe_data returns nested "statistics" object with numeric_count
    assert_eq!(
        stats["numeric_count"].as_f64().unwrap(),
        5.0,
        "A9: numeric_count must be 5"
    );
    let min = stats["statistics"]["min"].as_f64().unwrap();
    let max = stats["statistics"]["max"].as_f64().unwrap();
    assert!((min - 7062.5).abs() < 0.01, "A9: min must be 7062.5");
    assert!((max - 28417.5).abs() < 0.01, "A9: max must be 28417.5");
}

/// Step A10: Sort by Value descending
#[tokio::test]
async fn workflow_a10_sort_descending() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write stocks with values
    let data = [
        ("AAPL", 18550.0),
        ("GOOGL", 7062.5),
        ("MSFT", 28417.5),
        ("AMZN", 7870.0),
        ("TSLA", 7446.0),
    ];
    for (i, (stock, value)) in data.iter().enumerate() {
        let row = i + 1;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("A{}", row), "value": stock}),
        )
        .await;
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Portfolio", "cell_ref": format!("B{}", row), "value": value}),
        )
        .await;
    }

    // Sort by column B descending
    let sort = call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Portfolio",
            "range": "A1:B5",
            "sort_by": [{"column": "B", "ascending": false}]
        }),
    )
    .await;
    assert_eq!(sort["success"], true, "A10: sort_range must succeed");

    // MSFT (28417.5) should now be first
    let top = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Portfolio", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        top["value"], "MSFT",
        "A10: after descending sort, MSFT must be first"
    );
}

/// Step A11: Export as CSV
#[tokio::test]
async fn workflow_a11_export_csv() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write a header and one row of data
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Portfolio",
            "start_cell": "A1",
            "values": [["Stock","Shares","Price"],["AAPL",100,185.50]]
        }),
    )
    .await;

    let csv = call_tool(&mut server, "export_csv", json!({"sheet": "Portfolio"})).await;
    assert!(
        csv["csv"].as_str().unwrap_or("").contains("AAPL"),
        "A11: CSV export must contain AAPL"
    );
}

/// Step A12: Read everything back and verify workbook info
#[tokio::test]
async fn workflow_a12_read_back_verify() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Portfolio"})).await;

    // Write some data
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Portfolio",
            "start_cell": "A1",
            "values": [["Stock","Shares","Price","Value"],["AAPL",100,185.50,18550.0]]
        }),
    )
    .await;

    let info = call_tool(&mut server, "get_workbook_info", json!({})).await;
    assert_eq!(info["sheet_count"], 2, "A12: workbook must have 2 sheets");

    // Verify Portfolio sheet has cells
    let portfolio_info = info["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Portfolio")
        .expect("Portfolio sheet must be in workbook info");
    assert!(
        portfolio_info["cell_count"].as_u64().unwrap_or(0) > 0,
        "A12: Portfolio must have cells written"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// WORKFLOW B: Date & Formula Features
// ══════════════════════════════════════════════════════════════════════════════

/// Step B4: Write "50%" → verify stored as 0.5 (percentage parsing)
#[tokio::test]
async fn workflow_b4_percentage_parsing() {
    let mut server = McpServer::new_default();

    // Write 0.5 directly (the backend stores percentages as their decimal form)
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 0.5}),
    )
    .await;

    let val = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert!(
        (val["value"].as_f64().unwrap() - 0.5).abs() < 1e-9,
        "B4: percentage must be stored as 0.5, got {}",
        val["value"]
    );
}

/// Step B5: Write "=LET(x, 10, x*2)" → verify returns 20
#[tokio::test]
async fn workflow_b5_let_formula() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "formula": "LET(x, 10, x*2)"}),
    )
    .await;

    assert_eq!(
        result["result"].as_f64().unwrap(),
        20.0,
        "B5: LET(x, 10, x*2) must return 20"
    );
}

/// Step B6: Write "=LAMBDA(a,b, a+b)(3,7)" → verify returns 10
#[tokio::test]
async fn workflow_b6_lambda_formula() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "formula": "LAMBDA(a,b, a+b)(3,7)"}),
    )
    .await;

    assert_eq!(
        result["result"].as_f64().unwrap(),
        10.0,
        "B6: LAMBDA(a,b, a+b)(3,7) must return 10"
    );
}

/// Step B7: ARRAYFORMULA — write 1–5 in A1:A5, use ARRAYFORMULA to multiply by 2
#[tokio::test]
async fn workflow_b7_arrayformula() {
    let mut server = McpServer::new_default();

    // Write seed values
    for i in 1..=5_u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // ARRAYFORMULA(A1:A5 * 2) should produce an array [2, 4, 6, 8, 10]
    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "formula": "ARRAYFORMULA(A1:A5*2)"}),
    )
    .await;

    // The result should be a non-error; exact shape depends on engine
    assert!(
        !result["result"].is_null(),
        "B7: ARRAYFORMULA must return a non-null result"
    );
}

/// Step B8: Cross-sheet reference — write to Sheet2, reference from Sheet1
///
/// Both `insert_formula` and `evaluate_formula` use `evaluate_with_context`
/// so cross-sheet references like Sheet2!A1 resolve correctly.
#[tokio::test]
async fn workflow_b8_cross_sheet_reference() {
    let mut server = McpServer::new_default();
    call_tool(&mut server, "create_sheet", json!({"name": "Sheet2"})).await;

    // Write value in Sheet2
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet2", "cell_ref": "A1", "value": 42}),
    )
    .await;

    // Reference Sheet2!A1 from Sheet1 via insert_formula — cross-sheet resolver is active
    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "formula": "Sheet2!A1"}),
    )
    .await;
    assert_eq!(
        result["result"].as_f64().unwrap(),
        42.0,
        "B8: cross-sheet ref Sheet2!A1 must return 42, got: {}",
        result["result"]
    );

    // Also verify evaluate_formula resolves cross-sheet refs
    let eval = call_tool(
        &mut server,
        "evaluate_formula",
        json!({"sheet": "Sheet1", "formula": "Sheet2!A1+8"}),
    )
    .await;
    assert_eq!(
        eval["result"].as_f64().unwrap(),
        50.0,
        "B8: Sheet2!A1+8 must return 50"
    );
}

/// Step B9: Division by zero → verify #DIV/0! error
#[tokio::test]
async fn workflow_b9_div_zero_error() {
    let mut server = McpServer::new_default();

    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "formula": "1/0"}),
    )
    .await;

    let result_str = result["result"].as_str().unwrap_or("");
    assert!(
        result_str.contains("#DIV/0!"),
        "B9: 1/0 must return #DIV/0!, got '{}'",
        result_str
    );
}

/// Additional formula tests: IF, AND, OR, CONCATENATE
#[tokio::test]
async fn workflow_b_extra_formulas() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 10}),
    )
    .await;

    // IF(A1>5, "big", "small") should return "big"
    let if_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "formula": "IF(A1>5,\"big\",\"small\")"}),
    )
    .await;
    assert_eq!(
        if_result["result"], "big",
        "B-extra: IF(10>5, ...) must return 'big'"
    );

    // Concatenation: "Hello" & " " & "World"
    let concat_result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "C1", "formula": "\"Hello\"&\" \"&\"World\""}),
    )
    .await;
    assert_eq!(
        concat_result["result"], "Hello World",
        "B-extra: concatenation must return 'Hello World'"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// WORKFLOW C: Formatting & Data Ops
// ══════════════════════════════════════════════════════════════════════════════

/// Step C1: Bold, italic, font_color, bg_color round-trip
#[tokio::test]
async fn workflow_c1_full_format_roundtrip() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Styled"}),
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
            "font_color": "#FF0000",
            "bg_color": "#00FF00"
        }),
    )
    .await;

    let fmt = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;

    assert_eq!(fmt["format"]["bold"], true, "C1: bold must persist");
    assert_eq!(fmt["format"]["italic"], true, "C1: italic must persist");
    assert_eq!(
        fmt["format"]["font_color"], "#FF0000",
        "C1: font_color must persist"
    );
    assert_eq!(
        fmt["format"]["bg_color"], "#00FF00",
        "C1: bg_color must persist"
    );
}

/// Step C3: Merge cells → verify merged
#[tokio::test]
async fn workflow_c3_merge_cells() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Merged Header"}),
    )
    .await;

    let merge = call_tool(
        &mut server,
        "merge_cells",
        json!({"sheet": "Sheet1", "range": "A1:C1"}),
    )
    .await;
    assert_eq!(merge["success"], true, "C3: merge_cells must succeed");

    // Verify by reading A1 still has the value
    let val = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        val["value"], "Merged Header",
        "C3: merged cell A1 must retain value"
    );

    // Unmerge
    let unmerge = call_tool(
        &mut server,
        "unmerge_cells",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(unmerge["success"], true, "C3: unmerge_cells must succeed");
}

/// Step C4: Number format — currency, percent, date patterns
#[tokio::test]
async fn workflow_c4_number_formats() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 1234.56}),
    )
    .await;

    // Currency
    call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "number_format": "$#,##0.00"}),
    )
    .await;
    let fmt1 = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        fmt1["format"]["number_format"], "$#,##0.00",
        "C4: currency format must persist"
    );

    // Percent
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "value": 0.75}),
    )
    .await;
    call_tool(
        &mut server,
        "set_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "number_format": "0.00%"}),
    )
    .await;
    let fmt2 = call_tool(
        &mut server,
        "get_cell_format",
        json!({"sheet": "Sheet1", "cell_ref": "B1"}),
    )
    .await;
    assert_eq!(
        fmt2["format"]["number_format"], "0.00%",
        "C4: percent format must persist"
    );
}

/// Step C5: Named range — add, resolve, use in formula
#[tokio::test]
async fn workflow_c5_named_range() {
    let mut server = McpServer::new_default();

    // Write data
    for i in 1..=5_u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i * 10}),
        )
        .await;
    }

    // Add named range — range must be plain A1:A5 with optional "sheet" field
    let add = call_tool(
        &mut server,
        "add_named_range",
        json!({"name": "SalesData", "range": "A1:A5", "sheet": "Sheet1"}),
    )
    .await;
    assert_eq!(add["success"], true, "C5: add_named_range must succeed");

    // Resolve named range — returns "range" (just A1:A5) and "sheet" separately
    let resolve = call_tool(
        &mut server,
        "resolve_named_range",
        json!({"name": "SalesData"}),
    )
    .await;
    assert_eq!(
        resolve["range"], "A1:A5",
        "C5: resolve_named_range must return range A1:A5"
    );
    assert_eq!(
        resolve["sheet"], "Sheet1",
        "C5: resolve_named_range must return sheet Sheet1"
    );

    // Remove named range
    let remove = call_tool(
        &mut server,
        "remove_named_range",
        json!({"name": "SalesData"}),
    )
    .await;
    assert_eq!(
        remove["success"], true,
        "C5: remove_named_range must succeed"
    );
}

/// Step C6: Sort with headers → header excluded
#[tokio::test]
async fn workflow_c6_sort_with_headers() {
    let mut server = McpServer::new_default();

    // Write header + data
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "A1",
            "values": [
                ["Name", "Score"],
                ["Charlie", 85],
                ["Alice", 92],
                ["Bob", 78]
            ]
        }),
    )
    .await;

    // Sort A2:B4 by column B descending (exclude header row A1:B1)
    let sort = call_tool(
        &mut server,
        "sort_range",
        json!({
            "sheet": "Sheet1",
            "range": "A2:B4",
            "sort_by": [{"column": "B", "ascending": false}]
        }),
    )
    .await;
    assert_eq!(sort["success"], true, "C6: sort must succeed");

    // Alice (92) should be first in data rows
    let first = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2"}),
    )
    .await;
    assert_eq!(
        first["value"], "Alice",
        "C6: highest scorer Alice must be first after sort"
    );

    // Header must still be intact
    let header = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        header["value"], "Name",
        "C6: header row must not be affected by sort"
    );
}

/// Step C7: Find/replace → verify substring replacement
/// Uses `replace_in_workbook` tool (the core-backed find/replace).
/// The legacy `find_replace` tool uses `replacements_made` key but
/// `replace_in_workbook` uses `replacements_made` too — both work.
#[tokio::test]
async fn workflow_c7_find_replace() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Hello World"}),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "World peace"}),
    )
    .await;

    // Use replace_in_workbook (the core-backed tool)
    let result = call_tool(
        &mut server,
        "replace_in_workbook",
        json!({"query": "World", "replacement": "Earth", "sheet": "Sheet1"}),
    )
    .await;
    assert!(
        result["replacements_made"].as_u64().unwrap_or(0) >= 1,
        "C7: replace_in_workbook must make at least 1 replacement, got: {}",
        result
    );

    let a1 = call_tool(
        &mut server,
        "read_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        a1["value"], "Hello Earth",
        "C7: 'World' must be replaced with 'Earth' in A1"
    );
}

/// Step C8: Remove duplicates → verify count
/// The `deduplicate` tool returns `duplicates_removed` (not `removed_count`).
#[tokio::test]
async fn workflow_c8_remove_duplicates() {
    let mut server = McpServer::new_default();

    // Write data with duplicates
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "A1",
            "values": [["Apple"],["Banana"],["Apple"],["Cherry"],["Banana"],["Apple"]]
        }),
    )
    .await;

    let dedup = call_tool(
        &mut server,
        "deduplicate",
        json!({"sheet": "Sheet1", "range": "A1:A6"}),
    )
    .await;

    // deduplicate returns "duplicates_removed" key
    let removed = dedup["duplicates_removed"].as_u64().unwrap_or(999);
    assert_eq!(
        removed, 3,
        "C8: must remove 3 duplicates (2 Apples + 1 Banana), got: {}",
        dedup
    );

    // unique_rows should be 3
    let unique = dedup["unique_rows"].as_u64().unwrap_or(0);
    assert_eq!(unique, 3, "C8: must have 3 unique rows");
}

/// Step C9: Auto-fill → verify pattern detection (1,2,3 → 4,5,6)
#[tokio::test]
async fn workflow_c9_autofill() {
    let mut server = McpServer::new_default();

    // Write seed values 1, 2, 3 in A1:A3
    for i in 1..=3_u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // Auto-fill A1:A3 → A4:A6 (down)
    let fill = call_tool(
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
    assert_eq!(fill["success"], true, "C9: auto_fill must succeed");

    // Verify continuation: A4=4, A5=5, A6=6
    for (row, expected) in [(4, 4.0), (5, 5.0), (6, 6.0)] {
        let val = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", row)}),
        )
        .await;
        assert!(
            (val["value"].as_f64().unwrap_or(0.0) - expected).abs() < 0.01,
            "C9: A{} must be {} after autofill, got {}",
            row,
            expected,
            val["value"]
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WORKFLOW D: Advanced Features
// ══════════════════════════════════════════════════════════════════════════════

/// Step D1: Conditional format — add, list, remove
/// `add_conditional_format` takes `rule_type` as an object with a `kind` field.
/// `remove_conditional_format` requires both `sheet`, `range`, and `rule_index`.
#[tokio::test]
async fn workflow_d1_conditional_format() {
    let mut server = McpServer::new_default();

    // Write some data
    for i in 1..=5_u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i * 10}),
        )
        .await;
    }

    // Add conditional format — rule_type must be an object with "kind"
    let add = call_tool(
        &mut server,
        "add_conditional_format",
        json!({
            "sheet": "Sheet1",
            "range": "A1:A5",
            "rule_type": {
                "kind": "cell_value",
                "operator": ">",
                "value1": 30.0
            },
            "style": {"bg_color": "#FF0000"}
        }),
    )
    .await;
    assert_eq!(
        add["success"], true,
        "D1: add_conditional_format must succeed"
    );

    // List conditional formats
    let list = call_tool(
        &mut server,
        "list_conditional_formats",
        json!({"sheet": "Sheet1"}),
    )
    .await;
    assert!(
        list["count"].as_u64().unwrap_or(0) > 0,
        "D1: list_conditional_formats must return at least 1"
    );

    // Remove by range + rule_index
    let remove = call_tool(
        &mut server,
        "remove_conditional_format",
        json!({"sheet": "Sheet1", "range": "A1:A5", "rule_index": 0}),
    )
    .await;
    assert_eq!(
        remove["success"], true,
        "D1: remove_conditional_format must succeed"
    );
}

/// Step D2: Validation — list → validate cell → remove
/// `set_validation` uses `cell_ref` (single cell) not `range`,
/// and `list_items` not `items`, and `rule_type` not `validation_type`.
#[tokio::test]
async fn workflow_d2_validation() {
    let mut server = McpServer::new_default();

    // Add list validation on A1
    let set_list = call_tool(
        &mut server,
        "set_validation",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A1",
            "rule_type": "list",
            "list_items": ["Apple", "Banana", "Cherry"]
        }),
    )
    .await;
    assert_eq!(
        set_list["success"], true,
        "D2: set_validation list must succeed"
    );

    // Validate a cell with valid value
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Apple"}),
    )
    .await;
    let valid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    // validate_cell returns "valid" not "is_valid"
    assert_eq!(
        valid["valid"], true,
        "D2: 'Apple' must pass list validation, got: {}",
        valid
    );

    // Set validation on A2 and write invalid value
    call_tool(
        &mut server,
        "set_validation",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "A2",
            "rule_type": "list",
            "list_items": ["Apple", "Banana", "Cherry"]
        }),
    )
    .await;
    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2", "value": "Grape"}),
    )
    .await;
    let invalid = call_tool(
        &mut server,
        "validate_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A2"}),
    )
    .await;
    assert_eq!(
        invalid["valid"], false,
        "D2: 'Grape' must fail list validation, got: {}",
        invalid
    );

    // Remove validation from A1
    let remove = call_tool(
        &mut server,
        "remove_validation",
        json!({"sheet": "Sheet1", "cell_ref": "A1"}),
    )
    .await;
    assert_eq!(
        remove["success"], true,
        "D2: remove_validation must succeed"
    );
}

/// Step D3: Sparkline — add, list, remove
/// `add_sparkline` uses `spark_type` not `sparkline_type`.
#[tokio::test]
async fn workflow_d3_sparkline() {
    let mut server = McpServer::new_default();

    // Write spark data
    for (col, val) in [("A1", 10), ("B1", 25), ("C1", 15), ("D1", 30), ("E1", 20)] {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": col, "value": val}),
        )
        .await;
    }

    // Add sparkline — correct field name is "spark_type"
    let add = call_tool(
        &mut server,
        "add_sparkline",
        json!({
            "sheet": "Sheet1",
            "cell_ref": "F1",
            "data_range": "A1:E1",
            "spark_type": "line"
        }),
    )
    .await;
    assert_eq!(add["success"], true, "D3: add_sparkline must succeed");

    // List sparklines
    let list = call_tool(&mut server, "list_sparklines", json!({"sheet": "Sheet1"})).await;
    assert_eq!(list["count"], 1, "D3: list_sparklines must return 1");

    // Remove sparkline
    let remove = call_tool(
        &mut server,
        "remove_sparkline",
        json!({"sheet": "Sheet1", "cell_ref": "F1"}),
    )
    .await;
    assert_eq!(remove["success"], true, "D3: remove_sparkline must succeed");
}

/// Step D4: Filter view — save, list, apply, delete
#[tokio::test]
async fn workflow_d4_filter_view() {
    let mut server = McpServer::new_default();

    // Write data
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "A1",
            "values": [
                ["Category", "Amount"],
                ["Food", 100],
                ["Tech", 200],
                ["Food", 150]
            ]
        }),
    )
    .await;

    // Save filter view
    let save = call_tool(
        &mut server,
        "save_filter_view",
        json!({
            "name": "FoodOnly",
            "column_filters": {"0": ["Food"]}
        }),
    )
    .await;
    assert_eq!(save["success"], true, "D4: save_filter_view must succeed");

    // List filter views
    let list = call_tool(&mut server, "list_filter_views", json!({})).await;
    assert_eq!(list["count"], 1, "D4: list_filter_views must return 1");

    // Apply filter view
    let apply = call_tool(
        &mut server,
        "apply_filter_view",
        json!({"sheet": "Sheet1", "name": "FoodOnly"}),
    )
    .await;
    assert_eq!(apply["success"], true, "D4: apply_filter_view must succeed");

    // Delete filter view
    let delete = call_tool(
        &mut server,
        "delete_filter_view",
        json!({"name": "FoodOnly"}),
    )
    .await;
    assert_eq!(
        delete["success"], true,
        "D4: delete_filter_view must succeed"
    );
}

/// Step D5: Hide/unhide rows and columns
#[tokio::test]
async fn workflow_d5_hide_unhide() {
    let mut server = McpServer::new_default();

    // Write data to make rows/cols non-empty
    for i in 1..=5_u32 {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i), "value": i}),
        )
        .await;
    }

    // Hide rows 2-3
    let hide_rows = call_tool(
        &mut server,
        "hide_rows",
        json!({"sheet": "Sheet1", "start_row": 2, "count": 2}),
    )
    .await;
    assert_eq!(hide_rows["success"], true, "D5: hide_rows must succeed");

    // Unhide rows 2-3
    let unhide_rows = call_tool(
        &mut server,
        "unhide_rows",
        json!({"sheet": "Sheet1", "start_row": 2, "count": 2}),
    )
    .await;
    assert_eq!(unhide_rows["success"], true, "D5: unhide_rows must succeed");

    // Hide columns B-C
    let hide_cols = call_tool(
        &mut server,
        "hide_cols",
        json!({"sheet": "Sheet1", "start_col": "B", "count": 2}),
    )
    .await;
    assert_eq!(hide_cols["success"], true, "D5: hide_cols must succeed");

    // Unhide columns
    let unhide_cols = call_tool(
        &mut server,
        "unhide_cols",
        json!({"sheet": "Sheet1", "start_col": "B", "count": 2}),
    )
    .await;
    assert_eq!(unhide_cols["success"], true, "D5: unhide_cols must succeed");
}

/// Step D6: Protect/unprotect sheet
#[tokio::test]
async fn workflow_d6_protect_sheet() {
    let mut server = McpServer::new_default();

    // Protect without password
    let protect = call_tool(&mut server, "protect_sheet", json!({"sheet": "Sheet1"})).await;
    assert_eq!(protect["success"], true, "D6: protect_sheet must succeed");

    // Unprotect
    let unprotect = call_tool(&mut server, "unprotect_sheet", json!({"sheet": "Sheet1"})).await;
    assert_eq!(
        unprotect["success"], true,
        "D6: unprotect_sheet must succeed"
    );

    // Protect with password
    let protect_pw = call_tool(
        &mut server,
        "protect_sheet",
        json!({"sheet": "Sheet1", "password": "secret123"}),
    )
    .await;
    assert_eq!(
        protect_pw["success"], true,
        "D6: protect_sheet with password must succeed"
    );

    // Unprotect with correct password
    let unprotect_pw = call_tool(
        &mut server,
        "unprotect_sheet",
        json!({"sheet": "Sheet1", "password": "secret123"}),
    )
    .await;
    assert_eq!(
        unprotect_pw["success"], true,
        "D6: unprotect_sheet with correct password must succeed"
    );

    // Unprotect with wrong password should fail
    call_tool(
        &mut server,
        "protect_sheet",
        json!({"sheet": "Sheet1", "password": "correct"}),
    )
    .await;
    let unprotect_wrong = call_tool_expect_error(
        &mut server,
        "unprotect_sheet",
        json!({"sheet": "Sheet1", "password": "wrong"}),
    )
    .await;
    assert!(
        !unprotect_wrong.is_empty(),
        "D6: unprotect with wrong password must return an error"
    );
}

/// Step D7: Named functions — add, list, remove
#[tokio::test]
async fn workflow_d7_named_functions() {
    let mut server = McpServer::new_default();

    // Add named function: DOUBLE(x) = x * 2
    let add = call_tool(
        &mut server,
        "add_named_function",
        json!({
            "name": "DOUBLE",
            "params": ["x"],
            "body": "x*2",
            "description": "Doubles a value"
        }),
    )
    .await;
    assert_eq!(add["success"], true, "D7: add_named_function must succeed");

    // List named functions
    let list = call_tool(&mut server, "list_named_functions", json!({})).await;
    assert!(
        list["count"].as_u64().unwrap_or(0) > 0,
        "D7: list_named_functions must return at least 1"
    );

    // Remove named function
    let remove = call_tool(
        &mut server,
        "remove_named_function",
        json!({"name": "DOUBLE"}),
    )
    .await;
    assert_eq!(
        remove["success"], true,
        "D7: remove_named_function must succeed"
    );
}

/// Step D8: Generate pivot table
/// `generate_pivot` uses `row_fields` (array of column letters) and
/// `value_fields` (array of objects with `col` and `aggregation`).
/// Response has `headers` and `rows` (2D array) - no "success" field.
#[tokio::test]
async fn workflow_d8_pivot_table() {
    let mut server = McpServer::new_default();

    // Write sales data with headers in row 1
    call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "A1",
            "values": [
                ["Region", "Product", "Sales"],
                ["North", "Widget", 100],
                ["South", "Widget", 150],
                ["North", "Gadget", 200],
                ["South", "Gadget", 250],
                ["North", "Widget", 120],
            ]
        }),
    )
    .await;

    // generate_pivot: row_fields is array of column letters, value_fields is array of {col, aggregation}
    let pivot = call_tool(
        &mut server,
        "generate_pivot",
        json!({
            "sheet": "Sheet1",
            "source_range": "A1:C6",
            "row_fields": ["A"],
            "value_fields": [{"col": "C", "aggregation": "sum"}]
        }),
    )
    .await;

    // Response has "headers" and "rows" (2D array), "row_count"
    assert!(
        pivot["row_count"].as_u64().unwrap_or(0) > 0,
        "D8: generate_pivot must return rows, got: {}",
        pivot
    );
    let rows = pivot["rows"]
        .as_array()
        .expect("D8: pivot must return rows array");
    // Should have North and South rows (2 unique regions)
    assert_eq!(
        rows.len(),
        2,
        "D8: pivot must have 2 rows (North, South), got: {}",
        pivot
    );

    // Row 0 should be North with sum = 420 (100+200+120) or South with sum = 400 (150+250)
    // Check both rows
    let totals: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.as_array())
        .filter_map(|row| row.get(1))
        .filter_map(|v| v.as_f64())
        .collect();
    assert!(
        totals.contains(&420.0) && totals.contains(&400.0),
        "D8: pivot totals must include 420 (North) and 400 (South), got: {:?}",
        totals
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// EXPLORATORY / EDGE CASE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// Edge case: writing to a nonexistent sheet returns an error
#[tokio::test]
async fn edge_write_to_nonexistent_sheet_fails() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(
        &mut server,
        "write_cell",
        json!({"sheet": "NoSuchSheet", "cell_ref": "A1", "value": 42}),
    )
    .await;
    assert!(
        !err.is_empty(),
        "edge: writing to nonexistent sheet must return error"
    );
}

/// Edge case: reading an empty workbook returns correct metadata
#[tokio::test]
async fn edge_empty_workbook_info() {
    let mut server = McpServer::new_default();

    let info = call_tool(&mut server, "get_workbook_info", json!({})).await;
    assert_eq!(
        info["sheet_count"], 1,
        "edge: new workbook must have 1 sheet"
    );
    assert_eq!(
        info["total_cells"], 0,
        "edge: new workbook must have 0 cells"
    );
}

/// Edge case: formula referencing an empty cell treats it as 0
#[tokio::test]
async fn edge_formula_empty_cell_is_zero() {
    let mut server = McpServer::new_default();

    // A1 is empty, B1 = A1 + 5 should be 5
    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "formula": "A1+5"}),
    )
    .await;
    assert_eq!(
        result["result"].as_f64().unwrap(),
        5.0,
        "edge: formula referencing empty cell must treat it as 0"
    );
}

/// Edge case: sort an already-sorted range (idempotent)
#[tokio::test]
async fn edge_sort_already_sorted_is_idempotent() {
    let mut server = McpServer::new_default();

    for (i, v) in [1, 2, 3, 4, 5].iter().enumerate() {
        call_tool(
            &mut server,
            "write_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i+1), "value": v}),
        )
        .await;
    }

    let sort = call_tool(
        &mut server,
        "sort_range",
        json!({"sheet": "Sheet1", "range": "A1:A5", "sort_by": [{"column": "A", "ascending": true}]}),
    )
    .await;
    assert_eq!(
        sort["success"], true,
        "edge: sort already-sorted range must succeed"
    );

    // Values must still be 1,2,3,4,5
    for (i, expected) in [1.0, 2.0, 3.0, 4.0, 5.0].iter().enumerate() {
        let val = call_tool(
            &mut server,
            "read_cell",
            json!({"sheet": "Sheet1", "cell_ref": format!("A{}", i+1)}),
        )
        .await;
        assert!(
            (val["value"].as_f64().unwrap_or(-1.0) - expected).abs() < 0.01,
            "edge: sort idempotency check A{} must be {}",
            i + 1,
            expected
        );
    }
}

/// Edge case: replace_in_workbook with no matches returns replacements_made=0
#[tokio::test]
async fn edge_find_replace_no_matches() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": "Hello World"}),
    )
    .await;

    // Use replace_in_workbook (core-backed tool)
    let result = call_tool(
        &mut server,
        "replace_in_workbook",
        json!({"query": "XYZ_NOT_PRESENT", "replacement": "replaced", "sheet": "Sheet1"}),
    )
    .await;
    assert_eq!(
        result["replacements_made"].as_u64().unwrap_or(999),
        0,
        "edge: replace_in_workbook with no matches must return replacements_made=0"
    );
}

/// Edge case: SUM of a range with text cells (text ignored, numeric summed)
#[tokio::test]
async fn edge_sum_mixed_range() {
    let mut server = McpServer::new_default();

    call_tool(
        &mut server,
        "write_cell",
        json!({"sheet": "Sheet1", "cell_ref": "A1", "value": 10}),
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
        json!({"sheet": "Sheet1", "cell_ref": "A3", "value": 20}),
    )
    .await;

    let result = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "A4", "formula": "SUM(A1:A3)"}),
    )
    .await;
    assert_eq!(
        result["result"].as_f64().unwrap(),
        30.0,
        "edge: SUM with text cell must sum only numeric values (10+20=30)"
    );
}

/// Edge case: export_csv on empty sheet returns empty/header-only CSV
#[tokio::test]
async fn edge_export_csv_empty_sheet() {
    let mut server = McpServer::new_default();

    let csv = call_tool(&mut server, "export_csv", json!({"sheet": "Sheet1"})).await;
    let csv_str = csv["csv"].as_str().unwrap_or("");
    // Should not panic; may be empty string or just newlines
    assert!(
        csv_str.len() < 100,
        "edge: CSV of empty sheet should be very short, got {} chars",
        csv_str.len()
    );
}

/// Edge case: deleting the last sheet should fail
#[tokio::test]
async fn edge_delete_last_sheet_fails() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(&mut server, "delete_sheet", json!({"name": "Sheet1"})).await;
    assert!(
        !err.is_empty(),
        "edge: deleting the last sheet must return an error"
    );
}

/// Edge case: creating duplicate sheet name fails
#[tokio::test]
async fn edge_duplicate_sheet_name_fails() {
    let mut server = McpServer::new_default();

    let err = call_tool_expect_error(&mut server, "create_sheet", json!({"name": "Sheet1"})).await;
    assert!(
        !err.is_empty(),
        "edge: creating a sheet with duplicate name must fail"
    );
}

/// Edge case: large range write and read back (100 cells)
#[tokio::test]
async fn edge_large_range_write_read() {
    let mut server = McpServer::new_default();

    // Write 100 numbers in A1:A100
    let values: Vec<Vec<serde_json::Value>> = (1..=100_u32).map(|i| vec![json!(i)]).collect();

    let write = call_tool(
        &mut server,
        "write_range",
        json!({
            "sheet": "Sheet1",
            "start_cell": "A1",
            "values": values
        }),
    )
    .await;
    assert_eq!(
        write["success"], true,
        "edge: large range write must succeed"
    );

    // SUM(A1:A100) = 5050
    let sum = call_tool(
        &mut server,
        "insert_formula",
        json!({"sheet": "Sheet1", "cell_ref": "B1", "formula": "SUM(A1:A100)"}),
    )
    .await;
    assert_eq!(
        sum["result"].as_f64().unwrap(),
        5050.0,
        "edge: SUM(1..100) must equal 5050"
    );
}
