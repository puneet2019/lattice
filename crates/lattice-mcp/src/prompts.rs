//! MCP prompt templates.
//!
//! Prompts provide pre-built instruction templates for common spreadsheet tasks.
//! Each prompt returns a list of messages that the AI client can use to guide its work.

use serde::Deserialize;
use serde_json::{Value, json};

/// Handle the `prompts/list` method.
///
/// Returns available prompt templates with their argument schemas.
pub fn handle_list_prompts() -> Result<Value, (i32, String)> {
    Ok(json!({
        "prompts": [
            {
                "name": "analyze-portfolio",
                "description": "Analyze the investment portfolio in the current spreadsheet. Read all sheets, compute returns, risk metrics, and create visualizations.",
                "arguments": [],
            },
            {
                "name": "clean-data",
                "description": "Identify and fix data quality issues: missing values, duplicates, inconsistent formats, outliers.",
                "arguments": [
                    {
                        "name": "sheet",
                        "description": "Sheet to clean (defaults to active sheet)",
                        "required": false,
                    },
                    {
                        "name": "range",
                        "description": "Data range to clean (e.g. 'A1:F100'). If omitted, the entire used range is scanned.",
                        "required": false,
                    },
                ],
            },
            {
                "name": "create-dashboard",
                "description": "Create a summary dashboard with key metrics and charts from the current data.",
                "arguments": [
                    {
                        "name": "sheet",
                        "description": "Source data sheet (defaults to active sheet)",
                        "required": false,
                    },
                ],
            },
            {
                "name": "explain-formulas",
                "description": "List and explain all formulas in the current spreadsheet in plain language.",
                "arguments": [
                    {
                        "name": "sheet",
                        "description": "Sheet to analyze (defaults to active sheet)",
                        "required": false,
                    },
                ],
            },
            {
                "name": "financial-model",
                "description": "Build a financial model (revenue projections, expense tracking, cash flow) from the existing data.",
                "arguments": [
                    {
                        "name": "sheet",
                        "description": "Source data sheet",
                        "required": false,
                    },
                ],
            },
        ],
    }))
}

/// Arguments for prompts/get.
#[derive(Debug, Deserialize)]
pub struct GetPromptArgs {
    pub name: String,
    pub arguments: Option<Value>,
}

/// Handle the `prompts/get` method.
///
/// Returns the message list for a specific prompt template.
pub fn handle_get_prompt(params: Value) -> Result<Value, (i32, String)> {
    let args: GetPromptArgs = serde_json::from_value(params)
        .map_err(|e| (-32602, format!("Invalid arguments: {}", e)))?;

    let prompt_args = args.arguments.unwrap_or(json!({}));

    match args.name.as_str() {
        "analyze-portfolio" => Ok(prompt_analyze_portfolio()),
        "clean-data" => Ok(prompt_clean_data(&prompt_args)),
        "create-dashboard" => Ok(prompt_create_dashboard(&prompt_args)),
        "explain-formulas" => Ok(prompt_explain_formulas(&prompt_args)),
        "financial-model" => Ok(prompt_financial_model(&prompt_args)),
        _ => Err((-32602, format!("Unknown prompt: {}", args.name))),
    }
}

// ── Prompt Generators ────────────────────────────────────────────────────────

fn prompt_analyze_portfolio() -> Value {
    json!({
        "description": "Analyze the investment portfolio in the current spreadsheet",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": concat!(
                        "Analyze the investment portfolio in this spreadsheet. Follow these steps:\n\n",
                        "1. Use `list_sheets` to discover all sheets.\n",
                        "2. Use `read_range` on each sheet to understand the data structure.\n",
                        "3. Use `describe_data` to compute statistics on numerical columns.\n",
                        "4. Calculate portfolio metrics:\n",
                        "   - Total portfolio value\n",
                        "   - Individual position weights\n",
                        "   - Returns (if historical data exists)\n",
                        "   - Risk metrics (standard deviation, max drawdown)\n",
                        "5. Use `correlate` to find correlations between assets if applicable.\n",
                        "6. Use `create_sheet` to add an 'Analysis' sheet with results.\n",
                        "7. Use `write_range` to write the analysis results.\n",
                        "8. Use `create_chart` to visualize the portfolio allocation.\n",
                        "9. Provide a natural language summary of findings."
                    ),
                },
            },
        ],
    })
}

fn prompt_clean_data(args: &Value) -> Value {
    let sheet = args["sheet"].as_str().unwrap_or("the active sheet");
    let range = args["range"].as_str().unwrap_or("the entire used range");

    json!({
        "description": "Identify and fix data quality issues",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": format!(
                        concat!(
                            "Identify and fix data quality issues in {} of {}. Follow these steps:\n\n",
                            "1. Use `get_workbook_info` to understand the data.\n",
                            "2. Use `read_range` to examine the data.\n",
                            "3. Use `describe_data` to get statistics and identify anomalies.\n",
                            "4. Check for and fix these issues:\n",
                            "   a. **Missing values**: Use `find_replace` to locate empty cells. Suggest fill strategies.\n",
                            "   b. **Duplicates**: Use `deduplicate` to find and remove duplicate rows.\n",
                            "   c. **Inconsistent formats**: Look for mixed types in columns.\n",
                            "   d. **Outliers**: Use statistics to flag values beyond 3 standard deviations.\n",
                            "   e. **Whitespace/formatting**: Use `find_replace` with regex to clean text.\n",
                            "5. Use `write_cell` or `write_range` to apply fixes.\n",
                            "6. Report a summary of issues found and actions taken."
                        ),
                        range, sheet
                    ),
                },
            },
        ],
    })
}

fn prompt_create_dashboard(args: &Value) -> Value {
    let sheet = args["sheet"].as_str().unwrap_or("the active sheet");

    json!({
        "description": "Create a summary dashboard from the current data",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": format!(
                        concat!(
                            "Create a summary dashboard from the data in {}. Follow these steps:\n\n",
                            "1. Use `list_sheets` and `read_range` to understand the source data.\n",
                            "2. Use `describe_data` to compute key metrics.\n",
                            "3. Use `create_sheet` to create a 'Dashboard' sheet.\n",
                            "4. Write key metrics using `write_cell` with clear labels:\n",
                            "   - Totals, averages, counts\n",
                            "   - Min/max values\n",
                            "   - Percentages and ratios\n",
                            "5. Use `set_cell_format` to make headers bold and apply number formats.\n",
                            "6. Use `insert_formula` for metrics that should auto-update.\n",
                            "7. Use `create_chart` to add 2-3 relevant visualizations:\n",
                            "   - A summary chart (bar or pie)\n",
                            "   - A trend chart (line) if time-series data exists\n",
                            "   - A comparison chart if categories exist\n",
                            "8. Report what was created."
                        ),
                        sheet
                    ),
                },
            },
        ],
    })
}

fn prompt_explain_formulas(args: &Value) -> Value {
    let sheet = args["sheet"].as_str().unwrap_or("the active sheet");

    json!({
        "description": "List and explain all formulas in the spreadsheet",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": format!(
                        concat!(
                            "List and explain all formulas in {}. Follow these steps:\n\n",
                            "1. Use the `lattice://sheet/{{}}/formulas` resource to get all formulas.\n",
                            "2. For each formula found:\n",
                            "   a. Use `get_formula` to get the exact formula text.\n",
                            "   b. Use `read_cell` to see the computed value.\n",
                            "   c. Explain in plain language what the formula does.\n",
                            "   d. Note any cell references and what they point to.\n",
                            "3. Identify patterns:\n",
                            "   - Groups of related formulas\n",
                            "   - Potential circular references\n",
                            "   - Formulas that could be simplified\n",
                            "4. Provide a summary table with: Cell, Formula, Explanation, Current Value."
                        ),
                        sheet
                    ),
                },
            },
        ],
    })
}

fn prompt_financial_model(args: &Value) -> Value {
    let sheet = args["sheet"].as_str().unwrap_or("the active sheet");

    json!({
        "description": "Build a financial model from existing data",
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": format!(
                        concat!(
                            "Build a financial model from the data in {}. Follow these steps:\n\n",
                            "1. Use `list_sheets` and `read_range` to understand the existing data.\n",
                            "2. Identify financial data: revenue, expenses, dates, categories.\n",
                            "3. Use `create_sheet` to create model sheets:\n",
                            "   a. **Revenue Model**: Use `trend_analysis` on historical data, project forward.\n",
                            "   b. **Expense Tracking**: Categorize and summarize expenses.\n",
                            "   c. **Cash Flow**: Compute net cash flow = revenue - expenses.\n",
                            "4. Use `insert_formula` for calculated fields.\n",
                            "5. Use `write_range` to populate the model.\n",
                            "6. Use `set_cell_format` to apply currency formatting.\n",
                            "7. Use `create_chart` to visualize:\n",
                            "   - Revenue trend and forecast\n",
                            "   - Expense breakdown\n",
                            "   - Cash flow over time\n",
                            "8. Use `describe_data` to summarize key financial metrics.\n",
                            "9. Provide a narrative summary of the financial outlook."
                        ),
                        sheet
                    ),
                },
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_prompts() {
        let result = handle_list_prompts().unwrap();
        let prompts = result["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 5);

        let names: Vec<&str> = prompts
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"analyze-portfolio"));
        assert!(names.contains(&"clean-data"));
        assert!(names.contains(&"create-dashboard"));
        assert!(names.contains(&"explain-formulas"));
        assert!(names.contains(&"financial-model"));
    }

    #[test]
    fn test_get_prompt_analyze_portfolio() {
        let result = handle_get_prompt(json!({
            "name": "analyze-portfolio"
        }))
        .unwrap();

        assert!(result["messages"].is_array());
        let messages = result["messages"].as_array().unwrap();
        assert!(!messages.is_empty());
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn test_get_prompt_clean_data_with_args() {
        let result = handle_get_prompt(json!({
            "name": "clean-data",
            "arguments": {"sheet": "Data", "range": "A1:F100"}
        }))
        .unwrap();

        let text = result["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("A1:F100"));
        assert!(text.contains("Data"));
    }

    #[test]
    fn test_get_prompt_unknown() {
        let result = handle_get_prompt(json!({
            "name": "nonexistent"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_prompt_create_dashboard() {
        let result = handle_get_prompt(json!({
            "name": "create-dashboard"
        }))
        .unwrap();

        assert!(result["messages"].is_array());
    }

    #[test]
    fn test_get_prompt_explain_formulas() {
        let result = handle_get_prompt(json!({
            "name": "explain-formulas",
            "arguments": {"sheet": "Summary"}
        }))
        .unwrap();

        let text = result["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("Summary"));
    }

    #[test]
    fn test_get_prompt_financial_model() {
        let result = handle_get_prompt(json!({
            "name": "financial-model"
        }))
        .unwrap();

        assert!(result["messages"].is_array());
    }
}
