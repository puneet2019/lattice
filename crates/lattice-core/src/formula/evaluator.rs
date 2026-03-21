use crate::cell::CellValue;
use crate::error::{LatticeError, Result};
use crate::formula::FormulaEngine;
use crate::formula::parser::{Token, tokenize};
use crate::selection::parse_cell_ref;
use crate::sheet::Sheet;

/// A simple formula evaluator that supports basic aggregate functions
/// (`SUM`, `AVERAGE`, `COUNT`, `MIN`, `MAX`) over rectangular ranges.
pub struct SimpleEvaluator;

impl FormulaEngine for SimpleEvaluator {
    fn evaluate(&self, formula: &str, sheet: &Sheet) -> Result<CellValue> {
        let tokens = tokenize(formula);
        evaluate_tokens(&tokens, sheet)
    }
}

/// Resolve a range `start_ref:end_ref` to a flat list of numeric cell values
/// from the given sheet.
fn resolve_range(start_ref: &str, end_ref: &str, sheet: &Sheet) -> Result<Vec<f64>> {
    let start = parse_cell_ref(start_ref)?;
    let end = parse_cell_ref(end_ref)?;

    let r_min = start.row.min(end.row);
    let r_max = start.row.max(end.row);
    let c_min = start.col.min(end.col);
    let c_max = start.col.max(end.col);

    let mut values = Vec::new();
    for r in r_min..=r_max {
        for c in c_min..=c_max {
            if let Some(cell) = sheet.get_cell(r, c)
                && let CellValue::Number(n) = &cell.value
            {
                values.push(*n);
            }
        }
    }
    Ok(values)
}

/// Evaluate a tokenized formula.
///
/// Currently supports:
/// - `FUNC(CellRef:CellRef)` for SUM, AVERAGE, COUNT, MIN, MAX
/// - A bare numeric literal
/// - A bare cell reference
fn evaluate_tokens(tokens: &[Token], sheet: &Sheet) -> Result<CellValue> {
    if tokens.is_empty() {
        return Ok(CellValue::Empty);
    }

    // Pattern: Function ( CellRef : CellRef )
    if tokens.len() == 6
        && let (
            Token::Function(func),
            Token::LParen,
            Token::CellRef(start),
            Token::Colon,
            Token::CellRef(end),
            Token::RParen,
        ) = (
            &tokens[0], &tokens[1], &tokens[2], &tokens[3], &tokens[4], &tokens[5],
        )
    {
        let values = resolve_range(start, end, sheet)?;
        return apply_function(func, &values);
    }

    // Pattern: bare number
    if tokens.len() == 1 {
        if let Token::Number(n) = &tokens[0] {
            return Ok(CellValue::Number(*n));
        }
        // Pattern: bare cell reference
        if let Token::CellRef(r) = &tokens[0] {
            let cr = parse_cell_ref(r)?;
            return match sheet.get_cell(cr.row, cr.col) {
                Some(cell) => Ok(cell.value.clone()),
                None => Ok(CellValue::Empty),
            };
        }
    }

    Err(LatticeError::FormulaError(format!(
        "unsupported formula expression ({} tokens)",
        tokens.len()
    )))
}

/// Apply an aggregate function to a slice of numbers.
fn apply_function(name: &str, values: &[f64]) -> Result<CellValue> {
    match name {
        "SUM" => {
            let sum: f64 = values.iter().sum();
            Ok(CellValue::Number(sum))
        }
        "AVERAGE" => {
            if values.is_empty() {
                return Err(LatticeError::FormulaError(
                    "AVERAGE: no numeric values".into(),
                ));
            }
            let sum: f64 = values.iter().sum();
            Ok(CellValue::Number(sum / values.len() as f64))
        }
        "COUNT" => Ok(CellValue::Number(values.len() as f64)),
        "MIN" => {
            if values.is_empty() {
                return Ok(CellValue::Number(0.0));
            }
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            Ok(CellValue::Number(min))
        }
        "MAX" => {
            if values.is_empty() {
                return Ok(CellValue::Number(0.0));
            }
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            Ok(CellValue::Number(max))
        }
        _ => Err(LatticeError::FormulaError(format!(
            "unknown function: {name}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::sheet::Sheet;

    fn make_sheet_with_column(values: &[f64]) -> Sheet {
        let mut sheet = Sheet::new("Test");
        for (i, v) in values.iter().enumerate() {
            sheet.set_value(i as u32, 0, CellValue::Number(*v));
        }
        sheet
    }

    #[test]
    fn test_sum() {
        let sheet = make_sheet_with_column(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let eval = SimpleEvaluator;
        let result = eval.evaluate("SUM(A1:A5)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(15.0));
    }

    #[test]
    fn test_average() {
        let sheet = make_sheet_with_column(&[10.0, 20.0, 30.0]);
        let eval = SimpleEvaluator;
        let result = eval.evaluate("AVERAGE(A1:A3)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(20.0));
    }

    #[test]
    fn test_count() {
        let sheet = make_sheet_with_column(&[1.0, 2.0, 3.0]);
        let eval = SimpleEvaluator;
        let result = eval.evaluate("COUNT(A1:A3)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(3.0));
    }

    #[test]
    fn test_min() {
        let sheet = make_sheet_with_column(&[5.0, 3.0, 8.0]);
        let eval = SimpleEvaluator;
        let result = eval.evaluate("MIN(A1:A3)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(3.0));
    }

    #[test]
    fn test_max() {
        let sheet = make_sheet_with_column(&[5.0, 3.0, 8.0]);
        let eval = SimpleEvaluator;
        let result = eval.evaluate("MAX(A1:A3)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(8.0));
    }
}
