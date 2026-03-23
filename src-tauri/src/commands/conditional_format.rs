use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{
    ComparisonOperator, ConditionalRule, ConditionalRuleType, ConditionalStyle,
};

use crate::state::AppState;

/// Frontend-facing rule type description.
#[derive(Debug, Clone, Deserialize)]
pub struct RuleTypeInput {
    /// "cell_value", "text_contains", "is_blank", "is_not_blank", "is_error",
    /// "color_scale", "data_bar", "icon_set"
    pub kind: String,
    /// Comparison operator for cell_value rules: ">", "<", ">=", "<=", "=", "!=", "between"
    pub operator: Option<String>,
    /// First value for cell_value rules
    pub value1: Option<f64>,
    /// Second value for cell_value "between" rules
    pub value2: Option<f64>,
    /// Text needle for text_contains rules
    pub text: Option<String>,
    /// Minimum color for color_scale rules (CSS hex string)
    pub min_color: Option<String>,
    /// Maximum color for color_scale rules (CSS hex string)
    pub max_color: Option<String>,
    /// Optional midpoint color for color_scale rules (CSS hex string)
    pub mid_color: Option<String>,
    /// Bar color for data_bar rules (CSS hex string)
    pub bar_color: Option<String>,
    /// Icons for icon_set rules (e.g. ["↑", "→", "↓"])
    pub icons: Option<Vec<String>>,
    /// Thresholds for icon_set rules
    pub thresholds: Option<Vec<f64>>,
}

/// Frontend-facing style to apply when a rule matches.
#[derive(Debug, Clone, Deserialize)]
pub struct StyleInput {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
}

/// Serialized rule for listing.
#[derive(Debug, Clone, Serialize)]
pub struct RuleOutput {
    pub kind: String,
    pub description: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
    /// Comparison operator for cell_value rules (e.g. ">", "<", ">=").
    pub operator: Option<String>,
    /// First threshold value for cell_value rules.
    pub value1: Option<f64>,
    /// Second threshold value for cell_value "between" rules.
    pub value2: Option<f64>,
    /// Text needle for text_contains rules.
    pub text: Option<String>,
    /// Minimum color for color_scale rules.
    pub min_color: Option<String>,
    /// Maximum color for color_scale rules.
    pub max_color: Option<String>,
    /// Optional midpoint color for color_scale rules.
    pub mid_color: Option<String>,
    /// Bar color for data_bar rules.
    pub bar_color: Option<String>,
    /// Icons for icon_set rules.
    pub icons: Option<Vec<String>>,
    /// Thresholds for icon_set rules.
    pub thresholds: Option<Vec<f64>>,
}

/// Serialized conditional format range for listing.
#[derive(Debug, Clone, Serialize)]
pub struct ConditionalFormatOutput {
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
    pub rules: Vec<RuleOutput>,
}

/// Add a conditional formatting rule to a range.
#[tauri::command]
pub async fn add_conditional_format(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    rule_type: RuleTypeInput,
    style: StyleInput,
) -> Result<(), String> {
    let rule = parse_rule(rule_type, style)?;
    let mut store = state.conditional_formats.write().await;
    store.add_rule(&sheet, start_row, start_col, end_row, end_col, rule);
    Ok(())
}

/// List all conditional format ranges for a sheet.
#[tauri::command]
pub async fn list_conditional_formats(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<ConditionalFormatOutput>, String> {
    let store = state.conditional_formats.read().await;
    let ranges = store.list_ranges(&sheet);
    let output: Vec<ConditionalFormatOutput> = ranges
        .iter()
        .map(|r| ConditionalFormatOutput {
            start_row: r.start_row,
            start_col: r.start_col,
            end_row: r.end_row,
            end_col: r.end_col,
            rules: r.rules.iter().map(rule_to_output).collect(),
        })
        .collect();
    Ok(output)
}

/// Remove a conditional formatting rule by range coordinates and rule index.
#[tauri::command]
pub async fn remove_conditional_format(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    rule_index: usize,
) -> Result<(), String> {
    let mut store = state.conditional_formats.write().await;
    if store.remove_rule(&sheet, start_row, start_col, end_row, end_col, rule_index) {
        Ok(())
    } else {
        Err("Rule not found".to_string())
    }
}

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
                _ => return Err(format!("Unknown operator: {}", op_str)),
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
            icons: input.icons.unwrap_or_else(|| vec![
                "\u{2191}".to_string(),
                "\u{2192}".to_string(),
                "\u{2193}".to_string(),
            ]),
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

fn rule_to_output(rule: &ConditionalRule) -> RuleOutput {
    let (kind, description, operator, value1, value2, text) = match &rule.rule_type {
        ConditionalRuleType::CellValue { operator, value1, value2 } => {
            let op_str = match operator {
                ComparisonOperator::GreaterThan => ">",
                ComparisonOperator::LessThan => "<",
                ComparisonOperator::GreaterThanOrEqual => ">=",
                ComparisonOperator::LessThanOrEqual => "<=",
                ComparisonOperator::Equal => "=",
                ComparisonOperator::NotEqual => "!=",
                ComparisonOperator::Between => "between",
                ComparisonOperator::NotBetween => "not between",
            };
            let desc = if let Some(v2) = value2 {
                format!("Cell value {} {} and {}", op_str, value1, v2)
            } else {
                format!("Cell value {} {}", op_str, value1)
            };
            ("cell_value".to_string(), desc, Some(op_str.to_string()), Some(*value1), *value2, None)
        }
        ConditionalRuleType::TextContains(t) => {
            ("text_contains".to_string(), format!("Text contains \"{}\"", t), None, None, None, Some(t.clone()))
        }
        ConditionalRuleType::IsBlank => ("is_blank".to_string(), "Cell is blank".to_string(), None, None, None, None),
        ConditionalRuleType::IsNotBlank => ("is_not_blank".to_string(), "Cell is not blank".to_string(), None, None, None, None),
        ConditionalRuleType::IsError => ("is_error".to_string(), "Cell is error".to_string(), None, None, None, None),
        ConditionalRuleType::ColorScale { min_color, max_color, mid_color } => {
            return RuleOutput {
                kind: "color_scale".to_string(),
                description: format!("Color scale: {} to {}", min_color, max_color),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: Some(min_color.clone()),
                max_color: Some(max_color.clone()),
                mid_color: mid_color.clone(),
                bar_color: None,
                icons: None,
                thresholds: None,
            };
        }
        ConditionalRuleType::DataBar { color, .. } => {
            return RuleOutput {
                kind: "data_bar".to_string(),
                description: format!("Data bar: {}", color),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: None,
                max_color: None,
                mid_color: None,
                bar_color: Some(color.clone()),
                icons: None,
                thresholds: None,
            };
        }
        ConditionalRuleType::IconSet { icons, thresholds } => {
            return RuleOutput {
                kind: "icon_set".to_string(),
                description: format!("Icon set: {}", icons.join(" ")),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: None,
                max_color: None,
                mid_color: None,
                bar_color: None,
                icons: Some(icons.clone()),
                thresholds: Some(thresholds.clone()),
            };
        }
        _ => ("other".to_string(), "Custom rule".to_string(), None, None, None, None),
    };

    RuleOutput {
        kind,
        description,
        bold: rule.style.bold,
        italic: rule.style.italic,
        font_color: rule.style.font_color.clone(),
        bg_color: rule.style.bg_color.clone(),
        operator,
        value1,
        value2,
        text,
        min_color: None,
        max_color: None,
        mid_color: None,
        bar_color: None,
        icons: None,
        thresholds: None,
    }
}
