//! Conditional format MCP tool handlers: add_conditional_format, list_conditional_formats,
//! remove_conditional_format.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{
    CellRef, ComparisonOperator, ConditionalFormatStore, ConditionalRule, ConditionalRuleType,
    ConditionalStyle,
};

use super::ToolDef;
use crate::schema::{number_prop, object_schema, string_prop};

/// Return tool definitions for conditional format operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "add_conditional_format".to_string(),
            description: "Add a conditional formatting rule to a cell range. Supports cell_value comparisons, text_contains, is_blank, is_not_blank, is_error, color_scale, data_bar, and icon_set rules.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range in A1:B2 notation")),
                    (
                        "rule_type",
                        json!({
                            "type": "object",
                            "description": "Rule type specification",
                            "properties": {
                                "kind": {"type": "string", "description": "Rule kind: 'cell_value', 'text_contains', 'is_blank', 'is_not_blank', 'is_error', 'color_scale', 'data_bar', 'icon_set'"},
                                "operator": {"type": "string", "description": "Comparison operator for cell_value: '>', '<', '>=', '<=', '=', '!=', 'between', 'not_between'"},
                                "value1": {"type": "number", "description": "First threshold value for cell_value rules"},
                                "value2": {"type": "number", "description": "Second threshold for 'between' rules"},
                                "text": {"type": "string", "description": "Text needle for text_contains rules"},
                                "min_color": {"type": "string", "description": "Minimum color for color_scale (CSS hex)"},
                                "max_color": {"type": "string", "description": "Maximum color for color_scale (CSS hex)"},
                                "bar_color": {"type": "string", "description": "Bar color for data_bar (CSS hex)"}
                            },
                            "required": ["kind"]
                        }),
                    ),
                    (
                        "style",
                        json!({
                            "type": "object",
                            "description": "Style to apply when rule matches",
                            "properties": {
                                "bold": {"type": "boolean"},
                                "italic": {"type": "boolean"},
                                "font_color": {"type": "string", "description": "Font color (CSS hex)"},
                                "bg_color": {"type": "string", "description": "Background color (CSS hex)"}
                            }
                        }),
                    ),
                ],
                &["sheet", "range", "rule_type"],
            ),
        },
        ToolDef {
            name: "list_conditional_formats".to_string(),
            description: "List all conditional formatting rules for a sheet".to_string(),
            input_schema: object_schema(
                &[("sheet", string_prop("Sheet name"))],
                &["sheet"],
            ),
        },
        ToolDef {
            name: "remove_conditional_format".to_string(),
            description: "Remove a conditional formatting rule by range and rule index".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range in A1:B2 notation")),
                    ("rule_index", number_prop("Index of the rule to remove (0-based)")),
                ],
                &["sheet", "range", "rule_index"],
            ),
        },
    ]
}

// ── Rule type input ──────────────────────────────────────────────────────────

/// Rule type input from the MCP caller.
#[derive(Debug, Deserialize)]
pub struct RuleTypeInput {
    pub kind: String,
    pub operator: Option<String>,
    pub value1: Option<f64>,
    pub value2: Option<f64>,
    pub text: Option<String>,
    pub min_color: Option<String>,
    pub max_color: Option<String>,
    pub mid_color: Option<String>,
    pub bar_color: Option<String>,
    pub icons: Option<Vec<String>>,
    pub thresholds: Option<Vec<f64>>,
}

/// Style input from the MCP caller.
#[derive(Debug, Deserialize, Default)]
pub struct StyleInput {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Arguments for add_conditional_format.
#[derive(Debug, Deserialize)]
pub struct AddConditionalFormatArgs {
    pub sheet: String,
    pub range: String,
    pub rule_type: RuleTypeInput,
    pub style: Option<StyleInput>,
}

/// Handle the `add_conditional_format` tool call.
pub fn handle_add_conditional_format(
    store: &mut ConditionalFormatStore,
    args: Value,
) -> Result<Value, String> {
    let args: AddConditionalFormatArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;
    let style_input = args.style.unwrap_or_default();
    let rule = parse_rule(args.rule_type, style_input)?;

    store.add_rule(
        &args.sheet,
        start.row,
        start.col,
        end.row,
        end.col,
        rule,
    );

    Ok(json!({
        "success": true,
        "sheet": args.sheet,
        "range": args.range,
    }))
}

/// Arguments for list_conditional_formats.
#[derive(Debug, Deserialize)]
pub struct ListConditionalFormatsArgs {
    pub sheet: String,
}

/// Handle the `list_conditional_formats` tool call.
pub fn handle_list_conditional_formats(
    store: &ConditionalFormatStore,
    args: Value,
) -> Result<Value, String> {
    let args: ListConditionalFormatsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let ranges = store.list_ranges(&args.sheet);

    let output: Vec<Value> = ranges
        .iter()
        .map(|r| {
            let rules: Vec<Value> = r.rules.iter().map(rule_to_json).collect();
            json!({
                "range": format!(
                    "{}{}:{}{}",
                    lattice_core::col_to_letter(r.start_col),
                    r.start_row + 1,
                    lattice_core::col_to_letter(r.end_col),
                    r.end_row + 1
                ),
                "start_row": r.start_row + 1,
                "start_col": lattice_core::col_to_letter(r.start_col),
                "end_row": r.end_row + 1,
                "end_col": lattice_core::col_to_letter(r.end_col),
                "rules": rules,
            })
        })
        .collect();

    Ok(json!({
        "sheet": args.sheet,
        "conditional_formats": output,
        "count": output.len(),
    }))
}

/// Arguments for remove_conditional_format.
#[derive(Debug, Deserialize)]
pub struct RemoveConditionalFormatArgs {
    pub sheet: String,
    pub range: String,
    pub rule_index: usize,
}

/// Handle the `remove_conditional_format` tool call.
pub fn handle_remove_conditional_format(
    store: &mut ConditionalFormatStore,
    args: Value,
) -> Result<Value, String> {
    let args: RemoveConditionalFormatArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    let removed = store.remove_rule(
        &args.sheet,
        start.row,
        start.col,
        end.row,
        end.col,
        args.rule_index,
    );

    if removed {
        Ok(json!({
            "success": true,
            "sheet": args.sheet,
            "range": args.range,
        }))
    } else {
        Err(format!(
            "Rule not found at index {} for range '{}' on sheet '{}'",
            args.rule_index, args.range, args.sheet
        ))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a range string like "A1:C3" into two CellRefs.
fn parse_range(range: &str) -> Result<(CellRef, CellRef), String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format '{}': expected 'A1:B2'",
            range
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    Ok((start, end))
}

/// Parse a rule type input into a ConditionalRule.
fn parse_rule(input: RuleTypeInput, style_input: StyleInput) -> Result<ConditionalRule, String> {
    let rule_type = match input.kind.as_str() {
        "cell_value" => {
            let op_str = input.operator.as_deref().unwrap_or(">");
            let v1 = input.value1.unwrap_or(0.0);
            let v2 = input.value2;
            let operator = match op_str {
                ">" => ComparisonOperator::GreaterThan,
                "<" => ComparisonOperator::LessThan,
                ">=" => ComparisonOperator::GreaterThanOrEqual,
                "<=" => ComparisonOperator::LessThanOrEqual,
                "=" => ComparisonOperator::Equal,
                "!=" => ComparisonOperator::NotEqual,
                "between" => ComparisonOperator::Between,
                "not_between" => ComparisonOperator::NotBetween,
                _ => return Err(format!("Unknown comparison operator: {}", op_str)),
            };
            ConditionalRuleType::CellValue {
                operator,
                value1: v1,
                value2: v2,
            }
        }
        "text_contains" => {
            let text = input.text.unwrap_or_default();
            ConditionalRuleType::TextContains(text)
        }
        "is_blank" => ConditionalRuleType::IsBlank,
        "is_not_blank" => ConditionalRuleType::IsNotBlank,
        "is_error" => ConditionalRuleType::IsError,
        "color_scale" => ConditionalRuleType::ColorScale {
            min_color: input.min_color.unwrap_or_else(|| "#ffffff".to_string()),
            max_color: input.max_color.unwrap_or_else(|| "#ff0000".to_string()),
            mid_color: input.mid_color,
        },
        "data_bar" => ConditionalRuleType::DataBar {
            color: input.bar_color.unwrap_or_else(|| "#4285f4".to_string()),
            max_length_percent: 100,
        },
        "icon_set" => ConditionalRuleType::IconSet {
            icons: input
                .icons
                .unwrap_or_else(|| vec!["up".to_string(), "right".to_string(), "down".to_string()]),
            thresholds: input.thresholds.unwrap_or_else(|| vec![67.0, 33.0]),
        },
        _ => return Err(format!("Unknown rule kind: {}", input.kind)),
    };

    let style = ConditionalStyle {
        bold: style_input.bold,
        italic: style_input.italic,
        font_color: style_input.font_color,
        bg_color: style_input.bg_color,
    };

    Ok(ConditionalRule {
        rule_type,
        style,
        priority: 0,
        stop_if_true: false,
    })
}

/// Convert a ConditionalRule to a JSON value for output.
fn rule_to_json(rule: &ConditionalRule) -> Value {
    let kind = match &rule.rule_type {
        ConditionalRuleType::CellValue { operator, value1, value2 } => {
            let op_str = match operator {
                ComparisonOperator::GreaterThan => ">",
                ComparisonOperator::LessThan => "<",
                ComparisonOperator::GreaterThanOrEqual => ">=",
                ComparisonOperator::LessThanOrEqual => "<=",
                ComparisonOperator::Equal => "=",
                ComparisonOperator::NotEqual => "!=",
                ComparisonOperator::Between => "between",
                ComparisonOperator::NotBetween => "not_between",
            };
            json!({
                "kind": "cell_value",
                "operator": op_str,
                "value1": value1,
                "value2": value2,
            })
        }
        ConditionalRuleType::TextContains(t) => json!({"kind": "text_contains", "text": t}),
        ConditionalRuleType::TextStartsWith(t) => json!({"kind": "text_starts_with", "text": t}),
        ConditionalRuleType::TextEndsWith(t) => json!({"kind": "text_ends_with", "text": t}),
        ConditionalRuleType::IsBlank => json!({"kind": "is_blank"}),
        ConditionalRuleType::IsNotBlank => json!({"kind": "is_not_blank"}),
        ConditionalRuleType::IsError => json!({"kind": "is_error"}),
        ConditionalRuleType::DuplicateValues => json!({"kind": "duplicate_values"}),
        ConditionalRuleType::UniqueValues => json!({"kind": "unique_values"}),
        ConditionalRuleType::ColorScale { min_color, max_color, mid_color } => json!({
            "kind": "color_scale",
            "min_color": min_color,
            "max_color": max_color,
            "mid_color": mid_color,
        }),
        ConditionalRuleType::DataBar { color, max_length_percent } => json!({
            "kind": "data_bar",
            "color": color,
            "max_length_percent": max_length_percent,
        }),
        ConditionalRuleType::IconSet { icons, thresholds } => json!({
            "kind": "icon_set",
            "icons": icons,
            "thresholds": thresholds,
        }),
        ConditionalRuleType::Formula(f) => json!({"kind": "formula", "formula": f}),
    };

    json!({
        "rule_type": kind,
        "style": {
            "bold": rule.style.bold,
            "italic": rule.style.italic,
            "font_color": rule.style.font_color,
            "bg_color": rule.style.bg_color,
        },
        "priority": rule.priority,
        "stop_if_true": rule.stop_if_true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_conditional_format_cell_value() {
        let mut store = ConditionalFormatStore::new();
        let result = handle_add_conditional_format(
            &mut store,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B5",
                "rule_type": {"kind": "cell_value", "operator": ">", "value1": 100},
                "style": {"bold": true, "bg_color": "#FF0000"}
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        let ranges = store.list_ranges("Sheet1");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].rules.len(), 1);
    }

    #[test]
    fn test_add_conditional_format_text_contains() {
        let mut store = ConditionalFormatStore::new();
        handle_add_conditional_format(
            &mut store,
            json!({
                "sheet": "Sheet1",
                "range": "A1:A10",
                "rule_type": {"kind": "text_contains", "text": "error"},
                "style": {"font_color": "#FF0000"}
            }),
        )
        .unwrap();

        let ranges = store.list_ranges("Sheet1");
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_list_conditional_formats() {
        let mut store = ConditionalFormatStore::new();
        handle_add_conditional_format(
            &mut store,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B5",
                "rule_type": {"kind": "is_blank"},
            }),
        )
        .unwrap();

        let result = handle_list_conditional_formats(
            &store,
            json!({"sheet": "Sheet1"}),
        )
        .unwrap();

        assert_eq!(result["count"], 1);
        assert_eq!(result["conditional_formats"][0]["range"], "A1:B5");
    }

    #[test]
    fn test_remove_conditional_format() {
        let mut store = ConditionalFormatStore::new();
        handle_add_conditional_format(
            &mut store,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B5",
                "rule_type": {"kind": "is_blank"},
            }),
        )
        .unwrap();

        let result = handle_remove_conditional_format(
            &mut store,
            json!({"sheet": "Sheet1", "range": "A1:B5", "rule_index": 0}),
        )
        .unwrap();
        assert_eq!(result["success"], true);

        let ranges = store.list_ranges("Sheet1");
        assert_eq!(ranges.len(), 0);
    }

    #[test]
    fn test_remove_conditional_format_not_found() {
        let mut store = ConditionalFormatStore::new();
        let result = handle_remove_conditional_format(
            &mut store,
            json!({"sheet": "Sheet1", "range": "A1:B5", "rule_index": 0}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_invalid_rule_kind() {
        let mut store = ConditionalFormatStore::new();
        let result = handle_add_conditional_format(
            &mut store,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B5",
                "rule_type": {"kind": "nonexistent"},
            }),
        );
        assert!(result.is_err());
    }
}
