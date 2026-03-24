use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{ValidationRule, ValidationType};

use crate::state::AppState;

/// Serializable validation rule returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationData {
    /// The validation type: "list", "number", "date", "text_length", "custom".
    pub rule_type: String,
    /// For list validation: the comma-separated list items.
    pub list_items: Option<String>,
    /// For number/text_length: min value.
    pub min: Option<f64>,
    /// For number/text_length: max value.
    pub max: Option<f64>,
    /// For date: min date string.
    pub min_date: Option<String>,
    /// For date: max date string.
    pub max_date: Option<String>,
    /// For custom: the formula string.
    pub formula: Option<String>,
    /// Whether blank values are allowed.
    pub allow_blank: bool,
    /// Error message shown on validation failure.
    pub error_message: Option<String>,
}

/// Set a validation rule on a cell.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn set_validation(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
    rule_type: String,
    list_items: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    min_date: Option<String>,
    max_date: Option<String>,
    formula: Option<String>,
    allow_blank: Option<bool>,
    error_message: Option<String>,
) -> Result<(), String> {
    let validation_type = match rule_type.as_str() {
        "list" => {
            let items = list_items
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            ValidationType::List(items)
        }
        "number" => ValidationType::NumberRange { min, max },
        "date" => ValidationType::DateRange {
            min: min_date,
            max: max_date,
        },
        "text_length" => ValidationType::TextLength {
            min: min.map(|v| v as usize),
            max: max.map(|v| v as usize),
        },
        "custom" => ValidationType::Custom(formula.unwrap_or_default()),
        _ => return Err(format!("Unknown validation type: {}", rule_type)),
    };

    let rule = ValidationRule {
        validation_type,
        allow_blank: allow_blank.unwrap_or(true),
        error_message,
    };

    let mut wb = state.workbook.write().await;
    wb.validations.set_rule(&sheet, row, col, rule);
    Ok(())
}

/// Get the validation rule for a cell.
#[tauri::command]
pub async fn get_validation(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<Option<ValidationData>, String> {
    let wb = state.workbook.read().await;
    let rule = wb.validations.get_rule(&sheet, row, col);
    Ok(rule.map(rule_to_data))
}

/// Remove the validation rule from a cell.
#[tauri::command]
pub async fn remove_validation(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    wb.validations.remove_rule(&sheet, row, col);
    Ok(())
}

/// List all validation rules for a sheet, returning `(row, col, data)` tuples.
#[tauri::command]
pub async fn list_validations(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<(u32, u32, ValidationData)>, String> {
    let wb = state.workbook.read().await;
    let rules = wb.validations.list_rules(&sheet);
    Ok(rules
        .into_iter()
        .map(|((r, c), rule)| (r, c, rule_to_data(rule)))
        .collect())
}

/// Convert an internal `ValidationRule` to the serializable `ValidationData`.
fn rule_to_data(rule: &ValidationRule) -> ValidationData {
    match &rule.validation_type {
        ValidationType::List(items) => ValidationData {
            rule_type: "list".to_string(),
            list_items: Some(items.join(", ")),
            min: None,
            max: None,
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
        },
        ValidationType::NumberRange { min, max } => ValidationData {
            rule_type: "number".to_string(),
            list_items: None,
            min: *min,
            max: *max,
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
        },
        ValidationType::DateRange { min, max } => ValidationData {
            rule_type: "date".to_string(),
            list_items: None,
            min: None,
            max: None,
            min_date: min.clone(),
            max_date: max.clone(),
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
        },
        ValidationType::TextLength { min, max } => ValidationData {
            rule_type: "text_length".to_string(),
            list_items: None,
            min: min.map(|v| v as f64),
            max: max.map(|v| v as f64),
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
        },
        ValidationType::Custom(f) => ValidationData {
            rule_type: "custom".to_string(),
            list_items: None,
            min: None,
            max: None,
            min_date: None,
            max_date: None,
            formula: Some(f.clone()),
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
        },
    }
}
