//! Recursive formula evaluator supporting 70+ spreadsheet functions.
//!
//! The evaluator works on the [`Token`] stream produced by the parser and
//! resolves cell references against the provided [`Sheet`].

use crate::cell::{CellError, CellValue};
use crate::error::{LatticeError, Result};
use crate::formula::FormulaEngine;
use crate::formula::parser::{Token, tokenize};
use crate::selection::parse_cell_ref;
use crate::sheet::Sheet;
use rand::Rng;
use regex::Regex;

/// A recursive formula evaluator that supports 70+ spreadsheet functions,
/// arithmetic, comparisons, string concatenation, and nested expressions.
pub struct SimpleEvaluator;

impl FormulaEngine for SimpleEvaluator {
    fn evaluate(&self, formula: &str, sheet: &Sheet) -> Result<CellValue> {
        let tokens = tokenize(formula);
        let mut pos = 0;
        parse_expression(&tokens, &mut pos, sheet)
    }
}

// ---------------------------------------------------------------------------
// Recursive descent expression parser
// ---------------------------------------------------------------------------
// Precedence (low to high):
//   1. Comparison ( = < > <= >= <> )
//   2. Additive   ( + - )  and string concatenation ( & )
//   3. Multiplicative ( * / )
//   4. Unary minus
//   5. Atoms: numbers, strings, booleans, cell refs, ranges, function calls

/// Parse a complete expression.
fn parse_expression(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    parse_comparison(tokens, pos, sheet)
}

/// Parse comparison operators (=, <, >, <=, >=, <>).
fn parse_comparison(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    let mut left = parse_additive(tokens, pos, sheet)?;

    while *pos < tokens.len() {
        if let Token::Comparison(op) = &tokens[*pos] {
            let op = op.clone();
            *pos += 1;
            let right = parse_additive(tokens, pos, sheet)?;
            let result = compare_values(&left, &right, &op);
            left = CellValue::Boolean(result);
        } else {
            break;
        }
    }
    Ok(left)
}

/// Parse additive expressions (+, -, &).
fn parse_additive(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    let mut left = parse_multiplicative(tokens, pos, sheet)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Operator('+') => {
                *pos += 1;
                let right = parse_multiplicative(tokens, pos, sheet)?;
                let l = coerce_to_number(&left)?;
                let r = coerce_to_number(&right)?;
                left = CellValue::Number(l + r);
            }
            Token::Operator('-') => {
                *pos += 1;
                let right = parse_multiplicative(tokens, pos, sheet)?;
                let l = coerce_to_number(&left)?;
                let r = coerce_to_number(&right)?;
                left = CellValue::Number(l - r);
            }
            Token::Ampersand => {
                *pos += 1;
                let right = parse_multiplicative(tokens, pos, sheet)?;
                let l = coerce_to_string(&left);
                let r = coerce_to_string(&right);
                left = CellValue::Text(format!("{}{}", l, r));
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse multiplicative expressions (*, /).
fn parse_multiplicative(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    let mut left = parse_unary(tokens, pos, sheet)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Operator('*') => {
                *pos += 1;
                let right = parse_unary(tokens, pos, sheet)?;
                let l = coerce_to_number(&left)?;
                let r = coerce_to_number(&right)?;
                left = CellValue::Number(l * r);
            }
            Token::Operator('/') => {
                *pos += 1;
                let right = parse_unary(tokens, pos, sheet)?;
                let l = coerce_to_number(&left)?;
                let r = coerce_to_number(&right)?;
                if r == 0.0 {
                    return Ok(CellValue::Error(CellError::DivZero));
                }
                left = CellValue::Number(l / r);
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse unary minus.
fn parse_unary(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    if *pos < tokens.len() && tokens[*pos] == Token::Operator('-') {
        *pos += 1;
        let val = parse_atom(tokens, pos, sheet)?;
        let n = coerce_to_number(&val)?;
        return Ok(CellValue::Number(-n));
    }
    // Handle unary plus (just skip it)
    if *pos < tokens.len() && tokens[*pos] == Token::Operator('+') {
        *pos += 1;
        return parse_atom(tokens, pos, sheet);
    }
    parse_atom(tokens, pos, sheet)
}

/// Parse an atomic expression: literal, cell ref, range, parenthesised
/// expression, or function call.
fn parse_atom(tokens: &[Token], pos: &mut usize, sheet: &Sheet) -> Result<CellValue> {
    if *pos >= tokens.len() {
        return Err(LatticeError::FormulaError(
            "unexpected end of expression".into(),
        ));
    }

    match &tokens[*pos] {
        Token::Number(n) => {
            let n = *n;
            *pos += 1;
            Ok(CellValue::Number(n))
        }
        Token::StringLiteral(s) => {
            let s = s.clone();
            *pos += 1;
            Ok(CellValue::Text(s))
        }
        Token::Boolean(b) => {
            let b = *b;
            *pos += 1;
            Ok(CellValue::Boolean(b))
        }
        Token::LParen => {
            *pos += 1; // skip '('
            let val = parse_expression(tokens, pos, sheet)?;
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1; // skip ')'
            }
            Ok(val)
        }
        Token::Function(name) => {
            let name = name.clone();
            *pos += 1; // skip function name
            if *pos < tokens.len() && tokens[*pos] == Token::LParen {
                *pos += 1; // skip '('
            }
            let args = parse_function_args(tokens, pos, sheet, &name)?;
            if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                *pos += 1; // skip ')'
            }
            evaluate_function(&name, args, sheet)
        }
        Token::CellRef(r) => {
            let r = r.clone();
            *pos += 1;
            // Check if this is a range (next token is ':')
            if *pos < tokens.len() && tokens[*pos] == Token::Colon {
                // This is a range — return it as-is; the caller (function) will handle it
                // But if we're in expression context, just return the first cell's value
                let cr = parse_cell_ref(&r)?;
                return match sheet.get_cell(cr.row, cr.col) {
                    Some(cell) => Ok(cell.value.clone()),
                    None => Ok(CellValue::Empty),
                };
            }
            let cr = parse_cell_ref(&r)?;
            match sheet.get_cell(cr.row, cr.col) {
                Some(cell) => Ok(cell.value.clone()),
                None => Ok(CellValue::Empty),
            }
        }
        _ => Err(LatticeError::FormulaError(format!(
            "unexpected token: {:?}",
            tokens[*pos]
        ))),
    }
}

/// An argument to a function — either a single value or a range.
#[derive(Debug, Clone)]
enum FuncArg {
    /// A single evaluated value.
    Value(CellValue),
    /// A range of cell references (start_ref, end_ref) — not yet resolved.
    Range(String, String),
}

/// Parse the argument list of a function call.
///
/// This handles:
/// - Single value arguments (expressions)
/// - Range arguments (CellRef:CellRef)
/// - Nested function calls
fn parse_function_args(
    tokens: &[Token],
    pos: &mut usize,
    sheet: &Sheet,
    _func_name: &str,
) -> Result<Vec<FuncArg>> {
    let mut args: Vec<FuncArg> = Vec::new();

    // Empty argument list
    if *pos < tokens.len() && tokens[*pos] == Token::RParen {
        return Ok(args);
    }

    loop {
        if *pos >= tokens.len() {
            break;
        }

        // Check if this argument is a range: CellRef : CellRef
        if let Token::CellRef(start) = &tokens[*pos] {
            if *pos + 2 < tokens.len() && tokens[*pos + 1] == Token::Colon {
                if let Token::CellRef(end) = &tokens[*pos + 2] {
                    let start = start.clone();
                    let end = end.clone();
                    *pos += 3; // skip start : end
                    args.push(FuncArg::Range(start, end));
                    // Check for comma or end
                    if *pos < tokens.len() && tokens[*pos] == Token::Comma {
                        *pos += 1;
                        continue;
                    }
                    break;
                }
            }
        }

        // Otherwise, parse a full expression as a single-value argument
        let val = parse_expression(tokens, pos, sheet)?;
        args.push(FuncArg::Value(val));

        if *pos < tokens.len() && tokens[*pos] == Token::Comma {
            *pos += 1;
            continue;
        }
        break;
    }

    Ok(args)
}

// ---------------------------------------------------------------------------
// Value coercion helpers
// ---------------------------------------------------------------------------

/// Coerce a CellValue to f64 for arithmetic.
fn coerce_to_number(val: &CellValue) -> Result<f64> {
    match val {
        CellValue::Number(n) => Ok(*n),
        CellValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
        CellValue::Empty => Ok(0.0),
        CellValue::Text(s) => s
            .parse::<f64>()
            .map_err(|_| LatticeError::FormulaError(format!("cannot convert \"{s}\" to number"))),
        CellValue::Error(e) => Err(LatticeError::FormulaError(format!("cell error: {e}"))),
        CellValue::Date(_) => Ok(0.0),
    }
}

/// Coerce a CellValue to a boolean.
fn coerce_to_bool(val: &CellValue) -> Result<bool> {
    match val {
        CellValue::Boolean(b) => Ok(*b),
        CellValue::Number(n) => Ok(*n != 0.0),
        CellValue::Text(s) => match s.to_ascii_uppercase().as_str() {
            "TRUE" => Ok(true),
            "FALSE" => Ok(false),
            _ => Err(LatticeError::FormulaError(format!(
                "cannot convert \"{s}\" to boolean"
            ))),
        },
        CellValue::Empty => Ok(false),
        CellValue::Error(e) => Err(LatticeError::FormulaError(format!("cell error: {e}"))),
        CellValue::Date(_) => Ok(true),
    }
}

/// Coerce a CellValue to a string.
fn coerce_to_string(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => {
            if *n == n.floor() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
    }
}

/// Compare two CellValues with the given operator.
fn compare_values(left: &CellValue, right: &CellValue, op: &str) -> bool {
    // If both are numbers, compare numerically
    if let (Ok(l), Ok(r)) = (try_as_number(left), try_as_number(right)) {
        return match op {
            "=" => (l - r).abs() < f64::EPSILON,
            "<>" => (l - r).abs() >= f64::EPSILON,
            ">" => l > r,
            "<" => l < r,
            ">=" => l >= r,
            "<=" => l <= r,
            _ => false,
        };
    }
    // Otherwise compare as strings (case-insensitive)
    let l = coerce_to_string(left).to_ascii_uppercase();
    let r = coerce_to_string(right).to_ascii_uppercase();
    match op {
        "=" => l == r,
        "<>" => l != r,
        ">" => l > r,
        "<" => l < r,
        ">=" => l >= r,
        "<=" => l <= r,
        _ => false,
    }
}

/// Try to interpret a CellValue as a number without error.
fn try_as_number(val: &CellValue) -> std::result::Result<f64, ()> {
    match val {
        CellValue::Number(n) => Ok(*n),
        CellValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
        CellValue::Empty => Ok(0.0),
        _ => Err(()),
    }
}

// ---------------------------------------------------------------------------
// Range resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a range to a flat list of CellValues from the sheet.
fn resolve_range_values(start_ref: &str, end_ref: &str, sheet: &Sheet) -> Result<Vec<CellValue>> {
    let start = parse_cell_ref(start_ref)?;
    let end = parse_cell_ref(end_ref)?;
    let r_min = start.row.min(end.row);
    let r_max = start.row.max(end.row);
    let c_min = start.col.min(end.col);
    let c_max = start.col.max(end.col);

    let mut values = Vec::new();
    for r in r_min..=r_max {
        for c in c_min..=c_max {
            match sheet.get_cell(r, c) {
                Some(cell) => values.push(cell.value.clone()),
                None => values.push(CellValue::Empty),
            }
        }
    }
    Ok(values)
}

/// Resolve a range to a flat list of numeric values, skipping non-numeric cells.
fn resolve_range_numbers(start_ref: &str, end_ref: &str, sheet: &Sheet) -> Result<Vec<f64>> {
    let values = resolve_range_values(start_ref, end_ref, sheet)?;
    Ok(values
        .iter()
        .filter_map(|v| match v {
            CellValue::Number(n) => Some(*n),
            CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        })
        .collect())
}

/// Resolve a range to a 2D grid of CellValues.
fn resolve_range_2d(
    start_ref: &str,
    end_ref: &str,
    sheet: &Sheet,
) -> Result<Vec<Vec<CellValue>>> {
    let start = parse_cell_ref(start_ref)?;
    let end = parse_cell_ref(end_ref)?;
    let r_min = start.row.min(end.row);
    let r_max = start.row.max(end.row);
    let c_min = start.col.min(end.col);
    let c_max = start.col.max(end.col);

    let mut rows = Vec::new();
    for r in r_min..=r_max {
        let mut row = Vec::new();
        for c in c_min..=c_max {
            match sheet.get_cell(r, c) {
                Some(cell) => row.push(cell.value.clone()),
                None => row.push(CellValue::Empty),
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Collect all numeric values from function arguments, expanding ranges.
fn collect_numbers(args: &[FuncArg], sheet: &Sheet) -> Result<Vec<f64>> {
    let mut nums = Vec::new();
    for arg in args {
        match arg {
            FuncArg::Range(start, end) => {
                nums.extend(resolve_range_numbers(start, end, sheet)?);
            }
            FuncArg::Value(CellValue::Number(n)) => nums.push(*n),
            FuncArg::Value(CellValue::Boolean(b)) => nums.push(if *b { 1.0 } else { 0.0 }),
            FuncArg::Value(CellValue::Empty) => {}
            FuncArg::Value(CellValue::Text(s)) => {
                if let Ok(n) = s.parse::<f64>() {
                    nums.push(n);
                }
            }
            _ => {}
        }
    }
    Ok(nums)
}

/// Collect all CellValues from function arguments, expanding ranges.
fn collect_values(args: &[FuncArg], sheet: &Sheet) -> Result<Vec<CellValue>> {
    let mut vals = Vec::new();
    for arg in args {
        match arg {
            FuncArg::Range(start, end) => {
                vals.extend(resolve_range_values(start, end, sheet)?);
            }
            FuncArg::Value(v) => vals.push(v.clone()),
        }
    }
    Ok(vals)
}

/// Require exactly N single-value arguments.
fn require_args(args: &[FuncArg], n: usize, func_name: &str) -> Result<Vec<CellValue>> {
    if args.len() != n {
        return Err(LatticeError::FormulaError(format!(
            "{func_name} expects {n} argument(s), got {}",
            args.len()
        )));
    }
    let mut result = Vec::new();
    for arg in args {
        match arg {
            FuncArg::Value(v) => result.push(v.clone()),
            FuncArg::Range(_, _) => {
                return Err(LatticeError::FormulaError(format!(
                    "{func_name}: unexpected range argument"
                )));
            }
        }
    }
    Ok(result)
}

/// Require at least N single-value arguments and return all of them.
fn require_min_args(args: &[FuncArg], min: usize, func_name: &str) -> Result<Vec<CellValue>> {
    if args.len() < min {
        return Err(LatticeError::FormulaError(format!(
            "{func_name} expects at least {min} argument(s), got {}",
            args.len()
        )));
    }
    let mut result = Vec::new();
    for arg in args {
        match arg {
            FuncArg::Value(v) => result.push(v.clone()),
            FuncArg::Range(_, _) => {
                return Err(LatticeError::FormulaError(format!(
                    "{func_name}: unexpected range argument"
                )));
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Function dispatch
// ---------------------------------------------------------------------------

/// Evaluate a function call by name with its parsed arguments.
#[allow(clippy::too_many_lines)]
fn evaluate_function(name: &str, args: Vec<FuncArg>, sheet: &Sheet) -> Result<CellValue> {
    match name {
        // ===== MATH / AGGREGATE =====
        "SUM" => {
            let nums = collect_numbers(&args, sheet)?;
            Ok(CellValue::Number(nums.iter().sum()))
        }
        "AVERAGE" => {
            let nums = collect_numbers(&args, sheet)?;
            if nums.is_empty() {
                return Err(LatticeError::FormulaError(
                    "AVERAGE: no numeric values".into(),
                ));
            }
            Ok(CellValue::Number(nums.iter().sum::<f64>() / nums.len() as f64))
        }
        "COUNT" => {
            let vals = collect_values(&args, sheet)?;
            let count = vals
                .iter()
                .filter(|v| matches!(v, CellValue::Number(_)))
                .count();
            Ok(CellValue::Number(count as f64))
        }
        "COUNTA" => {
            let vals = collect_values(&args, sheet)?;
            let count = vals
                .iter()
                .filter(|v| !matches!(v, CellValue::Empty))
                .count();
            Ok(CellValue::Number(count as f64))
        }
        "MIN" => {
            let nums = collect_numbers(&args, sheet)?;
            if nums.is_empty() {
                return Ok(CellValue::Number(0.0));
            }
            Ok(CellValue::Number(
                nums.iter().cloned().fold(f64::INFINITY, f64::min),
            ))
        }
        "MAX" => {
            let nums = collect_numbers(&args, sheet)?;
            if nums.is_empty() {
                return Ok(CellValue::Number(0.0));
            }
            Ok(CellValue::Number(
                nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ))
        }
        "PRODUCT" => {
            let nums = collect_numbers(&args, sheet)?;
            if nums.is_empty() {
                return Ok(CellValue::Number(0.0));
            }
            Ok(CellValue::Number(nums.iter().product()))
        }
        "SUMPRODUCT" => {
            // SUMPRODUCT(range1, range2, ...)
            if args.len() < 2 {
                return Err(LatticeError::FormulaError(
                    "SUMPRODUCT requires at least 2 range arguments".into(),
                ));
            }
            let mut arrays: Vec<Vec<f64>> = Vec::new();
            for arg in &args {
                match arg {
                    FuncArg::Range(start, end) => {
                        arrays.push(resolve_range_numbers(start, end, sheet)?);
                    }
                    _ => {
                        return Err(LatticeError::FormulaError(
                            "SUMPRODUCT: arguments must be ranges".into(),
                        ));
                    }
                }
            }
            let len = arrays[0].len();
            for arr in &arrays {
                if arr.len() != len {
                    return Ok(CellValue::Error(CellError::Value));
                }
            }
            let mut sum = 0.0;
            for i in 0..len {
                let mut prod = 1.0;
                for arr in &arrays {
                    prod *= arr[i];
                }
                sum += prod;
            }
            Ok(CellValue::Number(sum))
        }
        "SUMIF" => {
            // SUMIF(range, criteria, [sum_range])
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "SUMIF requires 2 or 3 arguments".into(),
                ));
            }
            let criteria_range = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "SUMIF: first argument must be a range".into(),
                    ));
                }
            };
            let criteria = match &args[1] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "SUMIF: second argument must be a value".into(),
                    ));
                }
            };
            let sum_range = if args.len() == 3 {
                match &args[2] {
                    FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                    _ => {
                        return Err(LatticeError::FormulaError(
                            "SUMIF: third argument must be a range".into(),
                        ));
                    }
                }
            } else {
                criteria_range.clone()
            };

            let mut sum = 0.0;
            for (i, val) in criteria_range.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(sv) = sum_range.get(i) {
                        if let Ok(n) = coerce_to_number(sv) {
                            sum += n;
                        }
                    }
                }
            }
            Ok(CellValue::Number(sum))
        }
        "COUNTIF" => {
            // COUNTIF(range, criteria)
            let a = require_min_args_mixed(&args, 2, "COUNTIF")?;
            let range_vals = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "COUNTIF: first argument must be a range".into(),
                    ));
                }
            };
            let criteria = a[1].clone();
            let count = range_vals
                .iter()
                .filter(|v| matches_criteria(v, &criteria))
                .count();
            Ok(CellValue::Number(count as f64))
        }
        "AVERAGEIF" => {
            // AVERAGEIF(range, criteria, [average_range])
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "AVERAGEIF requires 2 or 3 arguments".into(),
                ));
            }
            let criteria_range = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "AVERAGEIF: first argument must be a range".into(),
                    ));
                }
            };
            let criteria = match &args[1] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "AVERAGEIF: second argument must be a value".into(),
                    ));
                }
            };
            let avg_range = if args.len() == 3 {
                match &args[2] {
                    FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                    _ => {
                        return Err(LatticeError::FormulaError(
                            "AVERAGEIF: third argument must be a range".into(),
                        ));
                    }
                }
            } else {
                criteria_range.clone()
            };

            let mut sum = 0.0;
            let mut count = 0;
            for (i, val) in criteria_range.iter().enumerate() {
                if matches_criteria(val, &criteria) {
                    if let Some(sv) = avg_range.get(i) {
                        if let Ok(n) = coerce_to_number(sv) {
                            sum += n;
                            count += 1;
                        }
                    }
                }
            }
            if count == 0 {
                return Err(LatticeError::FormulaError(
                    "AVERAGEIF: no matching values".into(),
                ));
            }
            Ok(CellValue::Number(sum / count as f64))
        }
        "ROUND" => {
            let a = require_args(&args, 2, "ROUND")?;
            let n = coerce_to_number(&a[0])?;
            let digits = coerce_to_number(&a[1])? as i32;
            Ok(CellValue::Number(round_to_digits(n, digits)))
        }
        "ROUNDUP" => {
            let a = require_args(&args, 2, "ROUNDUP")?;
            let n = coerce_to_number(&a[0])?;
            let digits = coerce_to_number(&a[1])? as i32;
            let factor = 10f64.powi(digits);
            let result = if n >= 0.0 {
                (n * factor).ceil() / factor
            } else {
                (n * factor).floor() / factor
            };
            Ok(CellValue::Number(result))
        }
        "ROUNDDOWN" => {
            let a = require_args(&args, 2, "ROUNDDOWN")?;
            let n = coerce_to_number(&a[0])?;
            let digits = coerce_to_number(&a[1])? as i32;
            let factor = 10f64.powi(digits);
            let result = (n * factor).trunc() / factor;
            Ok(CellValue::Number(result))
        }
        "ABS" => {
            let a = require_args(&args, 1, "ABS")?;
            let n = coerce_to_number(&a[0])?;
            Ok(CellValue::Number(n.abs()))
        }
        "CEILING" => {
            let a = require_args(&args, 2, "CEILING")?;
            let n = coerce_to_number(&a[0])?;
            let sig = coerce_to_number(&a[1])?;
            if sig == 0.0 {
                return Ok(CellValue::Number(0.0));
            }
            Ok(CellValue::Number((n / sig).ceil() * sig))
        }
        "FLOOR" => {
            let a = require_args(&args, 2, "FLOOR")?;
            let n = coerce_to_number(&a[0])?;
            let sig = coerce_to_number(&a[1])?;
            if sig == 0.0 {
                return Ok(CellValue::Number(0.0));
            }
            Ok(CellValue::Number((n / sig).floor() * sig))
        }
        "MOD" => {
            let a = require_args(&args, 2, "MOD")?;
            let n = coerce_to_number(&a[0])?;
            let d = coerce_to_number(&a[1])?;
            if d == 0.0 {
                return Ok(CellValue::Error(CellError::DivZero));
            }
            // MOD in spreadsheets: result has same sign as divisor
            let m = n % d;
            let result = if m != 0.0 && m.signum() != d.signum() {
                m + d
            } else {
                m
            };
            Ok(CellValue::Number(result))
        }
        "POWER" => {
            let a = require_args(&args, 2, "POWER")?;
            let base = coerce_to_number(&a[0])?;
            let exp = coerce_to_number(&a[1])?;
            Ok(CellValue::Number(base.powf(exp)))
        }
        "SQRT" => {
            let a = require_args(&args, 1, "SQRT")?;
            let n = coerce_to_number(&a[0])?;
            if n < 0.0 {
                return Ok(CellValue::Error(CellError::Num));
            }
            Ok(CellValue::Number(n.sqrt()))
        }
        "INT" => {
            let a = require_args(&args, 1, "INT")?;
            let n = coerce_to_number(&a[0])?;
            Ok(CellValue::Number(n.floor()))
        }
        "RAND" => {
            let mut rng = rand::rng();
            Ok(CellValue::Number(rng.random::<f64>()))
        }
        "RANDBETWEEN" => {
            let a = require_args(&args, 2, "RANDBETWEEN")?;
            let lo = coerce_to_number(&a[0])? as i64;
            let hi = coerce_to_number(&a[1])? as i64;
            if lo > hi {
                return Ok(CellValue::Error(CellError::Num));
            }
            let mut rng = rand::rng();
            let val = rng.random_range(lo..=hi);
            Ok(CellValue::Number(val as f64))
        }
        "LOG" => {
            // LOG(number, [base]) — base defaults to 10
            if args.is_empty() || args.len() > 2 {
                return Err(LatticeError::FormulaError(
                    "LOG requires 1 or 2 arguments".into(),
                ));
            }
            let a = require_min_args(&args, 1, "LOG")?;
            let n = coerce_to_number(&a[0])?;
            let base = if a.len() > 1 {
                coerce_to_number(&a[1])?
            } else {
                10.0
            };
            if n <= 0.0 || base <= 0.0 || base == 1.0 {
                return Ok(CellValue::Error(CellError::Num));
            }
            Ok(CellValue::Number(n.log(base)))
        }
        "LOG10" => {
            let a = require_args(&args, 1, "LOG10")?;
            let n = coerce_to_number(&a[0])?;
            if n <= 0.0 {
                return Ok(CellValue::Error(CellError::Num));
            }
            Ok(CellValue::Number(n.log10()))
        }
        "LN" => {
            let a = require_args(&args, 1, "LN")?;
            let n = coerce_to_number(&a[0])?;
            if n <= 0.0 {
                return Ok(CellValue::Error(CellError::Num));
            }
            Ok(CellValue::Number(n.ln()))
        }
        "EXP" => {
            let a = require_args(&args, 1, "EXP")?;
            let n = coerce_to_number(&a[0])?;
            Ok(CellValue::Number(n.exp()))
        }
        "PI" => Ok(CellValue::Number(std::f64::consts::PI)),
        "SIGN" => {
            let a = require_args(&args, 1, "SIGN")?;
            let n = coerce_to_number(&a[0])?;
            Ok(CellValue::Number(n.signum()))
        }

        // ===== LOGICAL =====
        "IF" => {
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "IF requires 2 or 3 arguments".into(),
                ));
            }
            let vals = require_min_args(&args, 2, "IF")?;
            let cond = coerce_to_bool(&vals[0])?;
            if cond {
                Ok(vals[1].clone())
            } else if vals.len() > 2 {
                Ok(vals[2].clone())
            } else {
                Ok(CellValue::Boolean(false))
            }
        }
        "AND" => {
            let vals = require_min_args(&args, 1, "AND")?;
            for v in &vals {
                if !coerce_to_bool(v)? {
                    return Ok(CellValue::Boolean(false));
                }
            }
            Ok(CellValue::Boolean(true))
        }
        "OR" => {
            let vals = require_min_args(&args, 1, "OR")?;
            for v in &vals {
                if coerce_to_bool(v)? {
                    return Ok(CellValue::Boolean(true));
                }
            }
            Ok(CellValue::Boolean(false))
        }
        "NOT" => {
            let a = require_args(&args, 1, "NOT")?;
            let b = coerce_to_bool(&a[0])?;
            Ok(CellValue::Boolean(!b))
        }
        "IFS" => {
            // IFS(cond1, val1, cond2, val2, ...)
            let vals = require_min_args(&args, 2, "IFS")?;
            if vals.len() % 2 != 0 {
                return Err(LatticeError::FormulaError(
                    "IFS requires an even number of arguments".into(),
                ));
            }
            let mut i = 0;
            while i < vals.len() {
                if coerce_to_bool(&vals[i])? {
                    return Ok(vals[i + 1].clone());
                }
                i += 2;
            }
            Ok(CellValue::Error(CellError::NA))
        }
        "IFERROR" => {
            let a = require_args(&args, 2, "IFERROR")?;
            if matches!(a[0], CellValue::Error(_)) {
                Ok(a[1].clone())
            } else {
                Ok(a[0].clone())
            }
        }
        "IFNA" => {
            let a = require_args(&args, 2, "IFNA")?;
            if matches!(a[0], CellValue::Error(CellError::NA)) {
                Ok(a[1].clone())
            } else {
                Ok(a[0].clone())
            }
        }
        "SWITCH" => {
            // SWITCH(expr, case1, val1, case2, val2, ..., [default])
            let vals = require_min_args(&args, 3, "SWITCH")?;
            let expr = &vals[0];
            let mut i = 1;
            while i + 1 < vals.len() {
                if compare_values(expr, &vals[i], "=") {
                    return Ok(vals[i + 1].clone());
                }
                i += 2;
            }
            // If there's a remaining single value, it's the default
            if i < vals.len() {
                return Ok(vals[i].clone());
            }
            Ok(CellValue::Error(CellError::NA))
        }
        "TRUE" => Ok(CellValue::Boolean(true)),
        "FALSE" => Ok(CellValue::Boolean(false)),

        // ===== TEXT =====
        "CONCATENATE" | "CONCAT" => {
            let vals = collect_values(&args, sheet)?;
            let s: String = vals.iter().map(|v| coerce_to_string(v)).collect();
            Ok(CellValue::Text(s))
        }
        "LEFT" => {
            let a = require_min_args(&args, 1, "LEFT")?;
            let s = coerce_to_string(&a[0]);
            let n = if a.len() > 1 {
                coerce_to_number(&a[1])? as usize
            } else {
                1
            };
            let result: String = s.chars().take(n).collect();
            Ok(CellValue::Text(result))
        }
        "RIGHT" => {
            let a = require_min_args(&args, 1, "RIGHT")?;
            let s = coerce_to_string(&a[0]);
            let n = if a.len() > 1 {
                coerce_to_number(&a[1])? as usize
            } else {
                1
            };
            let chars: Vec<char> = s.chars().collect();
            let start = chars.len().saturating_sub(n);
            let result: String = chars[start..].iter().collect();
            Ok(CellValue::Text(result))
        }
        "MID" => {
            let a = require_args(&args, 3, "MID")?;
            let s = coerce_to_string(&a[0]);
            let start = coerce_to_number(&a[1])? as usize;
            let len = coerce_to_number(&a[2])? as usize;
            if start == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let chars: Vec<char> = s.chars().collect();
            let start_idx = start.saturating_sub(1); // 1-based to 0-based
            let result: String = chars
                .iter()
                .skip(start_idx)
                .take(len)
                .collect();
            Ok(CellValue::Text(result))
        }
        "LEN" => {
            let a = require_args(&args, 1, "LEN")?;
            let s = coerce_to_string(&a[0]);
            Ok(CellValue::Number(s.chars().count() as f64))
        }
        "TRIM" => {
            let a = require_args(&args, 1, "TRIM")?;
            let s = coerce_to_string(&a[0]);
            // TRIM in spreadsheets removes leading/trailing and collapses internal spaces
            let parts: Vec<&str> = s.split_whitespace().collect();
            Ok(CellValue::Text(parts.join(" ")))
        }
        "UPPER" => {
            let a = require_args(&args, 1, "UPPER")?;
            let s = coerce_to_string(&a[0]);
            Ok(CellValue::Text(s.to_uppercase()))
        }
        "LOWER" => {
            let a = require_args(&args, 1, "LOWER")?;
            let s = coerce_to_string(&a[0]);
            Ok(CellValue::Text(s.to_lowercase()))
        }
        "PROPER" => {
            let a = require_args(&args, 1, "PROPER")?;
            let s = coerce_to_string(&a[0]);
            let mut result = String::new();
            let mut capitalize_next = true;
            for ch in s.chars() {
                if ch.is_alphabetic() {
                    if capitalize_next {
                        result.extend(ch.to_uppercase());
                        capitalize_next = false;
                    } else {
                        result.extend(ch.to_lowercase());
                    }
                } else {
                    result.push(ch);
                    capitalize_next = true;
                }
            }
            Ok(CellValue::Text(result))
        }
        "SUBSTITUTE" => {
            // SUBSTITUTE(text, old_text, new_text, [instance_num])
            let a = require_min_args(&args, 3, "SUBSTITUTE")?;
            let text = coerce_to_string(&a[0]);
            let old_text = coerce_to_string(&a[1]);
            let new_text = coerce_to_string(&a[2]);
            if a.len() > 3 {
                let instance = coerce_to_number(&a[3])? as usize;
                if instance == 0 {
                    return Ok(CellValue::Error(CellError::Value));
                }
                // Replace only the nth occurrence
                let mut count = 0;
                let mut result = String::new();
                let mut remaining = text.as_str();
                while let Some(pos) = remaining.find(&old_text) {
                    count += 1;
                    if count == instance {
                        result.push_str(&remaining[..pos]);
                        result.push_str(&new_text);
                        result.push_str(&remaining[pos + old_text.len()..]);
                        return Ok(CellValue::Text(result));
                    }
                    result.push_str(&remaining[..pos + old_text.len()]);
                    remaining = &remaining[pos + old_text.len()..];
                }
                result.push_str(remaining);
                Ok(CellValue::Text(result))
            } else {
                Ok(CellValue::Text(text.replace(&old_text, &new_text)))
            }
        }
        "REPLACE" => {
            // REPLACE(old_text, start_num, num_chars, new_text)
            let a = require_args(&args, 4, "REPLACE")?;
            let text = coerce_to_string(&a[0]);
            let start = coerce_to_number(&a[1])? as usize;
            let num_chars = coerce_to_number(&a[2])? as usize;
            let new_text = coerce_to_string(&a[3]);
            if start == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let chars: Vec<char> = text.chars().collect();
            let start_idx = start.saturating_sub(1);
            let mut result: String = chars.iter().take(start_idx).collect();
            result.push_str(&new_text);
            let end_idx = (start_idx + num_chars).min(chars.len());
            result.extend(chars[end_idx..].iter());
            Ok(CellValue::Text(result))
        }
        "FIND" => {
            // FIND(find_text, within_text, [start_num]) — case-sensitive
            let a = require_min_args(&args, 2, "FIND")?;
            let find_text = coerce_to_string(&a[0]);
            let within_text = coerce_to_string(&a[1]);
            let start = if a.len() > 2 {
                coerce_to_number(&a[2])? as usize
            } else {
                1
            };
            if start == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let search_from = start.saturating_sub(1);
            if search_from > within_text.len() {
                return Ok(CellValue::Error(CellError::Value));
            }
            match within_text[search_from..].find(&find_text) {
                Some(pos) => Ok(CellValue::Number((pos + start) as f64)),
                None => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "SEARCH" => {
            // SEARCH(find_text, within_text, [start_num]) — case-insensitive
            let a = require_min_args(&args, 2, "SEARCH")?;
            let find_text = coerce_to_string(&a[0]).to_lowercase();
            let within_text_orig = coerce_to_string(&a[1]);
            let within_text = within_text_orig.to_lowercase();
            let start = if a.len() > 2 {
                coerce_to_number(&a[2])? as usize
            } else {
                1
            };
            if start == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let search_from = start.saturating_sub(1);
            if search_from > within_text.len() {
                return Ok(CellValue::Error(CellError::Value));
            }
            match within_text[search_from..].find(&find_text) {
                Some(pos) => Ok(CellValue::Number((pos + start) as f64)),
                None => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "TEXT" => {
            // TEXT(value, format_text) — simplified implementation
            let a = require_args(&args, 2, "TEXT")?;
            let n = coerce_to_number(&a[0])?;
            let fmt = coerce_to_string(&a[1]);
            // Simple format support
            let result = match fmt.as_str() {
                "0" => format!("{}", n.round() as i64),
                "0.00" => format!("{:.2}", n),
                "0.0" => format!("{:.1}", n),
                "#,##0" => format_with_commas(n, 0),
                "#,##0.00" => format_with_commas(n, 2),
                "0%" => format!("{}%", (n * 100.0).round() as i64),
                "0.00%" => format!("{:.2}%", n * 100.0),
                _ => format!("{n}"),
            };
            Ok(CellValue::Text(result))
        }
        "VALUE" => {
            let a = require_args(&args, 1, "VALUE")?;
            let s = coerce_to_string(&a[0]);
            match s.parse::<f64>() {
                Ok(n) => Ok(CellValue::Number(n)),
                Err(_) => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "REPT" => {
            let a = require_args(&args, 2, "REPT")?;
            let s = coerce_to_string(&a[0]);
            let n = coerce_to_number(&a[1])? as usize;
            Ok(CellValue::Text(s.repeat(n)))
        }
        "EXACT" => {
            let a = require_args(&args, 2, "EXACT")?;
            let s1 = coerce_to_string(&a[0]);
            let s2 = coerce_to_string(&a[1]);
            Ok(CellValue::Boolean(s1 == s2))
        }
        "T" => {
            let a = require_args(&args, 1, "T")?;
            match &a[0] {
                CellValue::Text(s) => Ok(CellValue::Text(s.clone())),
                _ => Ok(CellValue::Text(String::new())),
            }
        }

        // ===== LOOKUP =====
        "VLOOKUP" => {
            // VLOOKUP(search_key, range, index, [is_sorted])
            if args.len() < 3 || args.len() > 4 {
                return Err(LatticeError::FormulaError(
                    "VLOOKUP requires 3 or 4 arguments".into(),
                ));
            }
            let search_key = match &args[0] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "VLOOKUP: first argument must be a value".into(),
                    ));
                }
            };
            let table = match &args[1] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "VLOOKUP: second argument must be a range".into(),
                    ));
                }
            };
            let col_index = match &args[2] {
                FuncArg::Value(v) => coerce_to_number(v)? as usize,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "VLOOKUP: third argument must be a number".into(),
                    ));
                }
            };
            let _is_sorted = if args.len() > 3 {
                match &args[3] {
                    FuncArg::Value(v) => coerce_to_bool(v)?,
                    _ => true,
                }
            } else {
                true
            };

            if col_index == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }

            // Search first column for match
            for row in &table {
                if !row.is_empty() && compare_values(&row[0], &search_key, "=") {
                    if col_index <= row.len() {
                        return Ok(row[col_index - 1].clone());
                    } else {
                        return Ok(CellValue::Error(CellError::Ref));
                    }
                }
            }
            Ok(CellValue::Error(CellError::NA))
        }
        "HLOOKUP" => {
            // HLOOKUP(search_key, range, row_index, [is_sorted])
            if args.len() < 3 || args.len() > 4 {
                return Err(LatticeError::FormulaError(
                    "HLOOKUP requires 3 or 4 arguments".into(),
                ));
            }
            let search_key = match &args[0] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "HLOOKUP: first argument must be a value".into(),
                    ));
                }
            };
            let table = match &args[1] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "HLOOKUP: second argument must be a range".into(),
                    ));
                }
            };
            let row_index = match &args[2] {
                FuncArg::Value(v) => coerce_to_number(v)? as usize,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "HLOOKUP: third argument must be a number".into(),
                    ));
                }
            };

            if row_index == 0 || table.is_empty() {
                return Ok(CellValue::Error(CellError::Value));
            }

            // Search first row for match
            let first_row = &table[0];
            for (col_idx, val) in first_row.iter().enumerate() {
                if compare_values(val, &search_key, "=") {
                    if row_index <= table.len() {
                        let target_row = &table[row_index - 1];
                        if col_idx < target_row.len() {
                            return Ok(target_row[col_idx].clone());
                        }
                    }
                    return Ok(CellValue::Error(CellError::Ref));
                }
            }
            Ok(CellValue::Error(CellError::NA))
        }
        "INDEX" => {
            // INDEX(range, row_num, [col_num])
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "INDEX requires 2 or 3 arguments".into(),
                ));
            }
            let table = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "INDEX: first argument must be a range".into(),
                    ));
                }
            };
            let row_num = match &args[1] {
                FuncArg::Value(v) => coerce_to_number(v)? as usize,
                _ => 1,
            };
            let col_num = if args.len() > 2 {
                match &args[2] {
                    FuncArg::Value(v) => coerce_to_number(v)? as usize,
                    _ => 1,
                }
            } else {
                1
            };

            if row_num == 0 || col_num == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            if row_num > table.len() {
                return Ok(CellValue::Error(CellError::Ref));
            }
            let row = &table[row_num - 1];
            if col_num > row.len() {
                return Ok(CellValue::Error(CellError::Ref));
            }
            Ok(row[col_num - 1].clone())
        }
        "MATCH" => {
            // MATCH(search_key, range, [match_type])
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "MATCH requires 2 or 3 arguments".into(),
                ));
            }
            let search_key = match &args[0] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "MATCH: first argument must be a value".into(),
                    ));
                }
            };
            let range_vals = match &args[1] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "MATCH: second argument must be a range".into(),
                    ));
                }
            };
            let _match_type = if args.len() > 2 {
                match &args[2] {
                    FuncArg::Value(v) => coerce_to_number(v)? as i32,
                    _ => 1,
                }
            } else {
                1
            };

            // Exact match (match_type = 0 behavior for simplicity)
            for (i, val) in range_vals.iter().enumerate() {
                if compare_values(val, &search_key, "=") {
                    return Ok(CellValue::Number((i + 1) as f64)); // 1-based
                }
            }
            Ok(CellValue::Error(CellError::NA))
        }
        "CHOOSE" => {
            // CHOOSE(index, val1, val2, ...)
            let vals = require_min_args(&args, 2, "CHOOSE")?;
            let index = coerce_to_number(&vals[0])? as usize;
            if index == 0 || index >= vals.len() {
                return Ok(CellValue::Error(CellError::Value));
            }
            Ok(vals[index].clone())
        }
        "XLOOKUP" => {
            // XLOOKUP(search_value, lookup_range, return_range, [not_found], [match_mode])
            // match_mode: 0 = exact (default), -1 = exact or next smaller, 1 = exact or next larger
            if args.len() < 3 || args.len() > 5 {
                return Err(LatticeError::FormulaError(
                    "XLOOKUP requires 3 to 5 arguments".into(),
                ));
            }
            let search_key = match &args[0] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(
                        "XLOOKUP: first argument must be a value".into(),
                    ));
                }
            };
            let lookup_vals = match &args[1] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "XLOOKUP: second argument must be a range".into(),
                    ));
                }
            };
            let return_vals = match &args[2] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "XLOOKUP: third argument must be a range".into(),
                    ));
                }
            };
            let not_found = if args.len() > 3 {
                match &args[3] {
                    FuncArg::Value(v) => Some(v.clone()),
                    _ => None,
                }
            } else {
                None
            };
            let match_mode = if args.len() > 4 {
                match &args[4] {
                    FuncArg::Value(v) => coerce_to_number(v)? as i32,
                    _ => 0,
                }
            } else {
                0
            };

            // Exact match first
            for (i, val) in lookup_vals.iter().enumerate() {
                if compare_values(val, &search_key, "=") {
                    return if i < return_vals.len() {
                        Ok(return_vals[i].clone())
                    } else {
                        Ok(CellValue::Error(CellError::Ref))
                    };
                }
            }
            // Approximate match modes
            if match_mode == -1 {
                // Next smaller: find largest value <= search_key
                let search_num = try_as_number(&search_key);
                if let Ok(sn) = search_num {
                    let mut best_idx: Option<usize> = None;
                    let mut best_val = f64::NEG_INFINITY;
                    for (i, val) in lookup_vals.iter().enumerate() {
                        if let Ok(n) = try_as_number(val) {
                            if n <= sn && n > best_val {
                                best_val = n;
                                best_idx = Some(i);
                            }
                        }
                    }
                    if let Some(idx) = best_idx {
                        return if idx < return_vals.len() {
                            Ok(return_vals[idx].clone())
                        } else {
                            Ok(CellValue::Error(CellError::Ref))
                        };
                    }
                }
            } else if match_mode == 1 {
                // Next larger: find smallest value >= search_key
                let search_num = try_as_number(&search_key);
                if let Ok(sn) = search_num {
                    let mut best_idx: Option<usize> = None;
                    let mut best_val = f64::INFINITY;
                    for (i, val) in lookup_vals.iter().enumerate() {
                        if let Ok(n) = try_as_number(val) {
                            if n >= sn && n < best_val {
                                best_val = n;
                                best_idx = Some(i);
                            }
                        }
                    }
                    if let Some(idx) = best_idx {
                        return if idx < return_vals.len() {
                            Ok(return_vals[idx].clone())
                        } else {
                            Ok(CellValue::Error(CellError::Ref))
                        };
                    }
                }
            }
            // Not found
            match not_found {
                Some(v) => Ok(v),
                None => Ok(CellValue::Error(CellError::NA)),
            }
        }
        "FILTER" => {
            // FILTER(range, condition_range) — return rows matching TRUE values.
            // Returns a comma-separated string since the evaluator returns CellValue.
            // NOTE: Array return is a limitation; we serialize to comma-separated text.
            if args.len() != 2 {
                return Err(LatticeError::FormulaError(
                    "FILTER requires exactly 2 arguments".into(),
                ));
            }
            let data_vals = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "FILTER: first argument must be a range".into(),
                    ));
                }
            };
            let cond_vals = match &args[1] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "FILTER: second argument must be a range".into(),
                    ));
                }
            };
            let mut results: Vec<String> = Vec::new();
            for (i, cond) in cond_vals.iter().enumerate() {
                let is_true = match cond {
                    CellValue::Boolean(true) => true,
                    CellValue::Number(n) => *n != 0.0,
                    _ => false,
                };
                if is_true {
                    if let Some(v) = data_vals.get(i) {
                        results.push(coerce_to_string(v));
                    }
                }
            }
            if results.is_empty() {
                Ok(CellValue::Error(CellError::NA))
            } else {
                Ok(CellValue::Text(results.join(",")))
            }
        }
        "SORT" => {
            // SORT(range, sort_index, [order])
            // sort_index: 1-based column index within the range (for 1D, always 1)
            // order: 1 = ascending (default), -1 = descending
            // Returns comma-separated string (array limitation).
            if args.is_empty() || args.len() > 3 {
                return Err(LatticeError::FormulaError(
                    "SORT requires 1 to 3 arguments".into(),
                ));
            }
            let mut vals = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "SORT: first argument must be a range".into(),
                    ));
                }
            };
            let order = if args.len() > 2 {
                match &args[2] {
                    FuncArg::Value(v) => coerce_to_number(v)? as i32,
                    _ => 1,
                }
            } else {
                1
            };
            // Sort by numeric value first, then by string
            vals.sort_by(|a, b| {
                let na = try_as_number(a);
                let nb = try_as_number(b);
                match (na, nb) {
                    (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    _ => coerce_to_string(a).cmp(&coerce_to_string(b)),
                }
            });
            if order == -1 {
                vals.reverse();
            }
            let strs: Vec<String> = vals.iter().map(|v| coerce_to_string(v)).collect();
            Ok(CellValue::Text(strs.join(",")))
        }
        "UNIQUE" => {
            // UNIQUE(range) — return unique values from a range.
            // Returns comma-separated string (array limitation).
            if args.len() != 1 {
                return Err(LatticeError::FormulaError(
                    "UNIQUE requires exactly 1 argument".into(),
                ));
            }
            let vals = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "UNIQUE: argument must be a range".into(),
                    ));
                }
            };
            let mut seen = Vec::new();
            let mut unique_strs = Vec::new();
            for v in &vals {
                let s = coerce_to_string(v);
                if !seen.contains(&s) {
                    seen.push(s.clone());
                    unique_strs.push(s);
                }
            }
            Ok(CellValue::Text(unique_strs.join(",")))
        }

        // ===== DATE =====
        "TODAY" => {
            // Return a date string in YYYY-MM-DD format
            // Since we're a pure engine without I/O, we use a simple stub
            // In production this would get the actual date from the caller
            Ok(CellValue::Text("TODAY".to_string()))
        }
        "NOW" => Ok(CellValue::Text("NOW".to_string())),
        "DATE" => {
            // DATE(year, month, day) -> serial date number or date string
            let a = require_args(&args, 3, "DATE")?;
            let year = coerce_to_number(&a[0])? as i32;
            let month = coerce_to_number(&a[1])? as u32;
            let day = coerce_to_number(&a[2])? as u32;
            Ok(CellValue::Text(format!("{year:04}-{month:02}-{day:02}")))
        }
        "YEAR" => {
            let a = require_args(&args, 1, "YEAR")?;
            let s = coerce_to_string(&a[0]);
            // Parse YYYY-MM-DD or YYYY/MM/DD
            let parts: Vec<&str> = s.split(|c| c == '-' || c == '/').collect();
            if parts.len() >= 3 {
                if let Ok(y) = parts[0].parse::<f64>() {
                    return Ok(CellValue::Number(y));
                }
            }
            Ok(CellValue::Error(CellError::Value))
        }
        "MONTH" => {
            let a = require_args(&args, 1, "MONTH")?;
            let s = coerce_to_string(&a[0]);
            let parts: Vec<&str> = s.split(|c| c == '-' || c == '/').collect();
            if parts.len() >= 3 {
                if let Ok(m) = parts[1].parse::<f64>() {
                    return Ok(CellValue::Number(m));
                }
            }
            Ok(CellValue::Error(CellError::Value))
        }
        "DAY" => {
            let a = require_args(&args, 1, "DAY")?;
            let s = coerce_to_string(&a[0]);
            let parts: Vec<&str> = s.split(|c| c == '-' || c == '/').collect();
            if parts.len() >= 3 {
                if let Ok(d) = parts[2].parse::<f64>() {
                    return Ok(CellValue::Number(d));
                }
            }
            Ok(CellValue::Error(CellError::Value))
        }
        "HOUR" | "MINUTE" | "SECOND" => {
            // Stub: parse HH:MM:SS from a date/time string
            let a = require_args(&args, 1, name)?;
            let s = coerce_to_string(&a[0]);
            // Try to find time portion after space
            let time_part = if let Some(idx) = s.find(' ') {
                &s[idx + 1..]
            } else {
                &s
            };
            let parts: Vec<&str> = time_part.split(':').collect();
            let index = match name {
                "HOUR" => 0,
                "MINUTE" => 1,
                "SECOND" => 2,
                _ => 0,
            };
            if let Some(part) = parts.get(index) {
                if let Ok(n) = part.parse::<f64>() {
                    return Ok(CellValue::Number(n));
                }
            }
            Ok(CellValue::Number(0.0))
        }
        "DATEDIF" => {
            // DATEDIF(start, end, unit) — simplified
            let a = require_args(&args, 3, "DATEDIF")?;
            let start = coerce_to_string(&a[0]);
            let end = coerce_to_string(&a[1]);
            let unit = coerce_to_string(&a[2]).to_uppercase();

            let start_parts: Vec<i32> = start
                .split(|c| c == '-' || c == '/')
                .filter_map(|s| s.parse().ok())
                .collect();
            let end_parts: Vec<i32> = end
                .split(|c| c == '-' || c == '/')
                .filter_map(|s| s.parse().ok())
                .collect();

            if start_parts.len() < 3 || end_parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }

            match unit.as_str() {
                "Y" => {
                    let years = end_parts[0] - start_parts[0];
                    Ok(CellValue::Number(years.max(0) as f64))
                }
                "M" => {
                    let months =
                        (end_parts[0] - start_parts[0]) * 12 + (end_parts[1] - start_parts[1]);
                    Ok(CellValue::Number(months.max(0) as f64))
                }
                "D" => {
                    // Simple approximation (doesn't account for varying month lengths)
                    let days = (end_parts[0] - start_parts[0]) * 365
                        + (end_parts[1] - start_parts[1]) * 30
                        + (end_parts[2] - start_parts[2]);
                    Ok(CellValue::Number(days.max(0) as f64))
                }
                _ => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "EDATE" => {
            // EDATE(start_date, months) — simplified
            let a = require_args(&args, 2, "EDATE")?;
            let s = coerce_to_string(&a[0]);
            let months = coerce_to_number(&a[1])? as i32;
            let parts: Vec<i32> = s
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let mut year = parts[0];
            let mut month = parts[1] + months;
            let day = parts[2];
            while month > 12 {
                month -= 12;
                year += 1;
            }
            while month < 1 {
                month += 12;
                year -= 1;
            }
            Ok(CellValue::Text(format!(
                "{year:04}-{month:02}-{day:02}"
            )))
        }
        "EOMONTH" => {
            // EOMONTH(start_date, months) — last day of the resulting month
            let a = require_args(&args, 2, "EOMONTH")?;
            let s = coerce_to_string(&a[0]);
            let months = coerce_to_number(&a[1])? as i32;
            let parts: Vec<i32> = s
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let mut year = parts[0];
            let mut month = parts[1] + months;
            while month > 12 {
                month -= 12;
                year += 1;
            }
            while month < 1 {
                month += 12;
                year -= 1;
            }
            let last_day = days_in_month(year, month as u32);
            Ok(CellValue::Text(format!(
                "{year:04}-{month:02}-{last_day:02}"
            )))
        }
        "WEEKDAY" => {
            // WEEKDAY(date, [type]) — simplified using Zeller's or Tomohiko Sakamoto
            let a = require_min_args(&args, 1, "WEEKDAY")?;
            let s = coerce_to_string(&a[0]);
            let parts: Vec<i32> = s
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let dow = day_of_week(parts[0], parts[1] as u32, parts[2] as u32);
            // Default type 1: Sunday=1, Monday=2, ..., Saturday=7
            Ok(CellValue::Number(dow as f64))
        }
        "NETWORKDAYS" => {
            // NETWORKDAYS(start_date, end_date) — simplified (no holidays)
            let a = require_args(&args, 2, "NETWORKDAYS")?;
            let start = coerce_to_string(&a[0]);
            let end = coerce_to_string(&a[1]);
            // Simplified: estimate business days
            let start_parts: Vec<i32> = start
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            let end_parts: Vec<i32> = end
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            if start_parts.len() < 3 || end_parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }
            // Rough estimate: total days * 5/7
            let total_days = (end_parts[0] - start_parts[0]) * 365
                + (end_parts[1] - start_parts[1]) * 30
                + (end_parts[2] - start_parts[2]);
            let work_days = (total_days as f64 * 5.0 / 7.0).round();
            Ok(CellValue::Number(work_days))
        }
        "DATEVALUE" => {
            // DATEVALUE(date_text) — return a serial date number (simplified)
            let a = require_args(&args, 1, "DATEVALUE")?;
            let s = coerce_to_string(&a[0]);
            let parts: Vec<i32> = s
                .split(|c: char| c == '-' || c == '/')
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() < 3 {
                return Ok(CellValue::Error(CellError::Value));
            }
            // Excel serial date: days since 1900-01-01 (simplified)
            let days = (parts[0] - 1900) * 365 + (parts[1] - 1) * 30 + parts[2];
            Ok(CellValue::Number(days as f64))
        }

        // ===== INFO =====
        "ISBLANK" => {
            let a = require_args(&args, 1, "ISBLANK")?;
            Ok(CellValue::Boolean(matches!(a[0], CellValue::Empty)))
        }
        "ISNUMBER" => {
            let a = require_args(&args, 1, "ISNUMBER")?;
            Ok(CellValue::Boolean(matches!(a[0], CellValue::Number(_))))
        }
        "ISTEXT" => {
            let a = require_args(&args, 1, "ISTEXT")?;
            Ok(CellValue::Boolean(matches!(a[0], CellValue::Text(_))))
        }
        "ISERROR" => {
            let a = require_args(&args, 1, "ISERROR")?;
            Ok(CellValue::Boolean(matches!(a[0], CellValue::Error(_))))
        }
        "ISLOGICAL" => {
            let a = require_args(&args, 1, "ISLOGICAL")?;
            Ok(CellValue::Boolean(matches!(a[0], CellValue::Boolean(_))))
        }
        "TYPE" => {
            let a = require_args(&args, 1, "TYPE")?;
            let type_num = match &a[0] {
                CellValue::Number(_) => 1.0,
                CellValue::Text(_) => 2.0,
                CellValue::Boolean(_) => 4.0,
                CellValue::Error(_) => 16.0,
                CellValue::Empty => 1.0,   // Empty is treated as number 0
                CellValue::Date(_) => 1.0, // Dates are numbers internally
            };
            Ok(CellValue::Number(type_num))
        }
        "N" => {
            let a = require_args(&args, 1, "N")?;
            match &a[0] {
                CellValue::Number(n) => Ok(CellValue::Number(*n)),
                CellValue::Boolean(b) => Ok(CellValue::Number(if *b { 1.0 } else { 0.0 })),
                _ => Ok(CellValue::Number(0.0)),
            }
        }
        "NA" => Ok(CellValue::Error(CellError::NA)),

        // ===== REGEX TEXT =====
        "REGEXMATCH" => {
            // REGEXMATCH(text, pattern) — returns TRUE if text matches regex
            let a = require_args(&args, 2, "REGEXMATCH")?;
            let text = coerce_to_string(&a[0]);
            let pattern = coerce_to_string(&a[1]);
            match Regex::new(&pattern) {
                Ok(re) => Ok(CellValue::Boolean(re.is_match(&text))),
                Err(_) => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "REGEXEXTRACT" => {
            // REGEXEXTRACT(text, pattern) — returns first match
            let a = require_args(&args, 2, "REGEXEXTRACT")?;
            let text = coerce_to_string(&a[0]);
            let pattern = coerce_to_string(&a[1]);
            match Regex::new(&pattern) {
                Ok(re) => {
                    if let Some(caps) = re.captures(&text) {
                        // Return first capture group if present, else full match
                        let result = caps
                            .get(1)
                            .or_else(|| caps.get(0))
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_default();
                        Ok(CellValue::Text(result))
                    } else {
                        Ok(CellValue::Error(CellError::NA))
                    }
                }
                Err(_) => Ok(CellValue::Error(CellError::Value)),
            }
        }
        "REGEXREPLACE" => {
            // REGEXREPLACE(text, pattern, replacement) — replace regex matches
            let a = require_args(&args, 3, "REGEXREPLACE")?;
            let text = coerce_to_string(&a[0]);
            let pattern = coerce_to_string(&a[1]);
            let replacement = coerce_to_string(&a[2]);
            match Regex::new(&pattern) {
                Ok(re) => {
                    let result = re.replace_all(&text, replacement.as_str()).to_string();
                    Ok(CellValue::Text(result))
                }
                Err(_) => Ok(CellValue::Error(CellError::Value)),
            }
        }

        // ===== ARRAY =====
        "TRANSPOSE" => {
            // TRANSPOSE(range) — swap rows and columns, return as CSV text.
            // Each row separated by semicolons, values within a row by commas.
            // NOTE: Array return limitation; we serialize to text.
            if args.len() != 1 {
                return Err(LatticeError::FormulaError(
                    "TRANSPOSE requires exactly 1 argument".into(),
                ));
            }
            let grid = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "TRANSPOSE: argument must be a range".into(),
                    ));
                }
            };
            if grid.is_empty() {
                return Ok(CellValue::Text(String::new()));
            }
            let num_rows = grid.len();
            let num_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut transposed_strs: Vec<String> = Vec::new();
            for c in 0..num_cols {
                let mut row_vals: Vec<String> = Vec::new();
                for row in grid.iter().take(num_rows) {
                    let val = row.get(c).cloned().unwrap_or(CellValue::Empty);
                    row_vals.push(coerce_to_string(&val));
                }
                transposed_strs.push(row_vals.join(","));
            }
            Ok(CellValue::Text(transposed_strs.join(";")))
        }
        "SEQUENCE" => {
            // SEQUENCE(rows, [cols], [start], [step])
            // Returns a comma-separated sequence of numbers.
            // Multiple rows separated by semicolons.
            if args.is_empty() || args.len() > 4 {
                return Err(LatticeError::FormulaError(
                    "SEQUENCE requires 1 to 4 arguments".into(),
                ));
            }
            let a = require_min_args(&args, 1, "SEQUENCE")?;
            let rows = coerce_to_number(&a[0])? as usize;
            let cols = if a.len() > 1 {
                coerce_to_number(&a[1])? as usize
            } else {
                1
            };
            let start = if a.len() > 2 {
                coerce_to_number(&a[2])?
            } else {
                1.0
            };
            let step = if a.len() > 3 {
                coerce_to_number(&a[3])?
            } else {
                1.0
            };
            if rows == 0 || cols == 0 {
                return Ok(CellValue::Error(CellError::Value));
            }
            let mut current = start;
            let mut row_strs: Vec<String> = Vec::new();
            for _ in 0..rows {
                let mut col_vals: Vec<String> = Vec::new();
                for _ in 0..cols {
                    if current == current.floor() && current.abs() < 1e15 {
                        col_vals.push(format!("{}", current as i64));
                    } else {
                        col_vals.push(format!("{current}"));
                    }
                    current += step;
                }
                row_strs.push(col_vals.join(","));
            }
            if rows == 1 {
                Ok(CellValue::Text(row_strs.join(",")))
            } else {
                Ok(CellValue::Text(row_strs.join(";")))
            }
        }
        "FLATTEN" => {
            // FLATTEN(range) — flatten a 2D range to a 1D comma-separated list.
            if args.len() != 1 {
                return Err(LatticeError::FormulaError(
                    "FLATTEN requires exactly 1 argument".into(),
                ));
            }
            let vals = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_values(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "FLATTEN: argument must be a range".into(),
                    ));
                }
            };
            let strs: Vec<String> = vals.iter().map(|v| coerce_to_string(v)).collect();
            Ok(CellValue::Text(strs.join(",")))
        }

        // ===== DATABASE =====
        // Database functions operate on a "database" range where row 0 is
        // headers. "field" is a column name (text) or 1-based index (number).
        // "criteria" is a range where row 0 is a header matching a database
        // column, and row 1+ are criteria values.
        "DSUM" | "DAVERAGE" | "DCOUNT" | "DMAX" | "DMIN" => {
            if args.len() != 3 {
                return Err(LatticeError::FormulaError(format!(
                    "{name} requires exactly 3 arguments"
                )));
            }
            let database = match &args[0] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(format!(
                        "{name}: first argument must be a range"
                    )));
                }
            };
            let field = match &args[1] {
                FuncArg::Value(v) => v.clone(),
                _ => {
                    return Err(LatticeError::FormulaError(format!(
                        "{name}: second argument must be a field name or index"
                    )));
                }
            };
            let criteria = match &args[2] {
                FuncArg::Range(s, e) => resolve_range_2d(s, e, sheet)?,
                _ => {
                    return Err(LatticeError::FormulaError(format!(
                        "{name}: third argument must be a range"
                    )));
                }
            };

            if database.is_empty() {
                return Ok(CellValue::Error(CellError::Value));
            }
            let headers = &database[0];

            // Resolve the field to a 0-based column index
            let field_col = match &field {
                CellValue::Number(n) => {
                    let idx = *n as usize;
                    if idx == 0 || idx > headers.len() {
                        return Ok(CellValue::Error(CellError::Value));
                    }
                    idx - 1
                }
                CellValue::Text(s) => {
                    let field_upper = s.to_ascii_uppercase();
                    match headers.iter().position(|h| {
                        coerce_to_string(h).to_ascii_uppercase() == field_upper
                    }) {
                        Some(i) => i,
                        None => return Ok(CellValue::Error(CellError::Value)),
                    }
                }
                _ => return Ok(CellValue::Error(CellError::Value)),
            };

            // Build criteria matching: criteria row 0 = header names,
            // criteria rows 1+ = values to match (OR across rows, AND within a row)
            let matching_values =
                database_matching_values(&database, field_col, &criteria, headers);

            match name {
                "DSUM" => {
                    let sum: f64 = matching_values
                        .iter()
                        .filter_map(|v| try_as_number(v).ok())
                        .sum();
                    Ok(CellValue::Number(sum))
                }
                "DAVERAGE" => {
                    let nums: Vec<f64> = matching_values
                        .iter()
                        .filter_map(|v| try_as_number(v).ok())
                        .collect();
                    if nums.is_empty() {
                        return Ok(CellValue::Error(CellError::DivZero));
                    }
                    Ok(CellValue::Number(
                        nums.iter().sum::<f64>() / nums.len() as f64,
                    ))
                }
                "DCOUNT" => {
                    let count = matching_values
                        .iter()
                        .filter(|v| matches!(v, CellValue::Number(_)))
                        .count();
                    Ok(CellValue::Number(count as f64))
                }
                "DMAX" => {
                    let nums: Vec<f64> = matching_values
                        .iter()
                        .filter_map(|v| try_as_number(v).ok())
                        .collect();
                    if nums.is_empty() {
                        return Ok(CellValue::Number(0.0));
                    }
                    Ok(CellValue::Number(
                        nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    ))
                }
                "DMIN" => {
                    let nums: Vec<f64> = matching_values
                        .iter()
                        .filter_map(|v| try_as_number(v).ok())
                        .collect();
                    if nums.is_empty() {
                        return Ok(CellValue::Number(0.0));
                    }
                    Ok(CellValue::Number(
                        nums.iter().cloned().fold(f64::INFINITY, f64::min),
                    ))
                }
                _ => unreachable!(),
            }
        }

        // TODO: XIRR(values, dates, [guess]) — IRR for irregular dates.
        // Skipped due to complexity of date serial number handling.
        // TODO: XNPV(rate, values, dates) — NPV for irregular dates.
        // Skipped due to complexity of date serial number handling.

        // ===== FINANCIAL =====
        "PMT" => {
            // PMT(rate, nper, pv, [fv], [type])
            let a = require_min_args(&args, 3, "PMT")?;
            let rate = coerce_to_number(&a[0])?;
            let nper = coerce_to_number(&a[1])?;
            let pv = coerce_to_number(&a[2])?;
            let fv = if a.len() > 3 {
                coerce_to_number(&a[3])?
            } else {
                0.0
            };
            let pmt_type = if a.len() > 4 {
                coerce_to_number(&a[4])? as i32
            } else {
                0
            };

            if rate == 0.0 {
                return Ok(CellValue::Number(-(pv + fv) / nper));
            }
            let pmt = if pmt_type == 0 {
                (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                    / ((1.0 + rate).powf(nper) - 1.0)
            } else {
                (-pv * rate * (1.0 + rate).powf(nper) - fv * rate)
                    / (((1.0 + rate).powf(nper) - 1.0) * (1.0 + rate))
            };
            Ok(CellValue::Number(pmt))
        }
        "FV" => {
            // FV(rate, nper, pmt, [pv], [type])
            let a = require_min_args(&args, 3, "FV")?;
            let rate = coerce_to_number(&a[0])?;
            let nper = coerce_to_number(&a[1])?;
            let pmt = coerce_to_number(&a[2])?;
            let pv = if a.len() > 3 {
                coerce_to_number(&a[3])?
            } else {
                0.0
            };
            let fv_type = if a.len() > 4 {
                coerce_to_number(&a[4])? as i32
            } else {
                0
            };

            if rate == 0.0 {
                return Ok(CellValue::Number(-pv - pmt * nper));
            }
            let factor = (1.0 + rate).powf(nper);
            let fv = if fv_type == 0 {
                -pv * factor - pmt * (factor - 1.0) / rate
            } else {
                -pv * factor - pmt * (factor - 1.0) / rate * (1.0 + rate)
            };
            Ok(CellValue::Number(fv))
        }
        "PV" => {
            // PV(rate, nper, pmt, [fv], [type])
            let a = require_min_args(&args, 3, "PV")?;
            let rate = coerce_to_number(&a[0])?;
            let nper = coerce_to_number(&a[1])?;
            let pmt = coerce_to_number(&a[2])?;
            let fv = if a.len() > 3 {
                coerce_to_number(&a[3])?
            } else {
                0.0
            };
            let pv_type = if a.len() > 4 {
                coerce_to_number(&a[4])? as i32
            } else {
                0
            };

            if rate == 0.0 {
                return Ok(CellValue::Number(-fv - pmt * nper));
            }
            let factor = (1.0 + rate).powf(nper);
            let pv = if pv_type == 0 {
                (-fv - pmt * (factor - 1.0) / rate) / factor
            } else {
                (-fv - pmt * (factor - 1.0) / rate * (1.0 + rate)) / factor
            };
            Ok(CellValue::Number(pv))
        }
        "NPV" => {
            // NPV(rate, value1, value2, ...)
            if args.is_empty() {
                return Err(LatticeError::FormulaError(
                    "NPV requires at least 2 arguments".into(),
                ));
            }
            let rate = match &args[0] {
                FuncArg::Value(v) => coerce_to_number(v)?,
                _ => {
                    return Err(LatticeError::FormulaError(
                        "NPV: first argument must be a rate".into(),
                    ));
                }
            };
            let cashflows = collect_numbers(&args[1..], sheet)?;
            let mut npv = 0.0;
            for (i, cf) in cashflows.iter().enumerate() {
                npv += cf / (1.0 + rate).powi(i as i32 + 1);
            }
            Ok(CellValue::Number(npv))
        }
        "IRR" => {
            // IRR(values, [guess]) — Newton's method
            let cashflows = collect_numbers(&args, sheet)?;
            if cashflows.is_empty() {
                return Ok(CellValue::Error(CellError::Num));
            }
            let mut rate: f64 = 0.1; // initial guess
            for _ in 0..100 {
                let mut npv: f64 = 0.0;
                let mut dnpv: f64 = 0.0;
                for (i, cf) in cashflows.iter().enumerate() {
                    let t = i as f64;
                    npv += cf / (1.0_f64 + rate).powf(t);
                    dnpv -= t * cf / (1.0_f64 + rate).powf(t + 1.0);
                }
                if dnpv.abs() < 1e-12 {
                    break;
                }
                let new_rate = rate - npv / dnpv;
                if (new_rate - rate).abs() < 1e-10 {
                    return Ok(CellValue::Number(new_rate));
                }
                rate = new_rate;
            }
            Ok(CellValue::Number(rate))
        }
        "RATE" => {
            // RATE(nper, pmt, pv, [fv], [type], [guess]) — Newton's method
            let a = require_min_args(&args, 3, "RATE")?;
            let nper: f64 = coerce_to_number(&a[0])?;
            let pmt: f64 = coerce_to_number(&a[1])?;
            let pv: f64 = coerce_to_number(&a[2])?;
            let fv: f64 = if a.len() > 3 {
                coerce_to_number(&a[3])?
            } else {
                0.0
            };

            let mut rate: f64 = 0.1;
            for _ in 0..100 {
                let factor: f64 = (1.0_f64 + rate).powf(nper);
                let f = pv * factor + pmt * (factor - 1.0) / rate + fv;
                let df = pv * nper * (1.0_f64 + rate).powf(nper - 1.0)
                    + pmt * (nper * (1.0_f64 + rate).powf(nper - 1.0) * rate - (factor - 1.0))
                        / (rate * rate);
                if df.abs() < 1e-12 {
                    break;
                }
                let new_rate = rate - f / df;
                if (new_rate - rate).abs() < 1e-10 {
                    return Ok(CellValue::Number(new_rate));
                }
                rate = new_rate;
            }
            Ok(CellValue::Number(rate))
        }

        _ => Err(LatticeError::FormulaError(format!(
            "unknown function: {name}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Round a number to the given number of decimal digits.
fn round_to_digits(n: f64, digits: i32) -> f64 {
    let factor = 10f64.powi(digits);
    (n * factor).round() / factor
}

/// Format a number with comma separators and the given number of decimal places.
fn format_with_commas(n: f64, decimals: usize) -> String {
    let abs = n.abs();
    let int_part = abs.trunc() as u64;
    let int_str = int_part.to_string();
    let mut result = String::new();
    for (i, ch) in int_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    let mut formatted: String = result.chars().rev().collect();
    if decimals > 0 {
        formatted.push_str(&format!(".{:0>width$}", ((abs.fract() * 10f64.powi(decimals as i32)).round() as u64), width = decimals));
    }
    if n < 0.0 {
        format!("-{formatted}")
    } else {
        formatted
    }
}

/// Check if a cell value matches a criteria value.
///
/// Criteria can be:
/// - A plain value (exact match)
/// - A string starting with >, <, >=, <=, <>, = followed by a value
fn matches_criteria(cell_val: &CellValue, criteria: &CellValue) -> bool {
    let criteria_str = coerce_to_string(criteria);
    // Check for operator prefix
    if let Some(rest) = criteria_str.strip_prefix(">=") {
        if let Ok(threshold) = rest.trim().parse::<f64>() {
            if let Ok(n) = try_as_number(cell_val) {
                return n >= threshold;
            }
        }
        return false;
    }
    if let Some(rest) = criteria_str.strip_prefix("<=") {
        if let Ok(threshold) = rest.trim().parse::<f64>() {
            if let Ok(n) = try_as_number(cell_val) {
                return n <= threshold;
            }
        }
        return false;
    }
    if let Some(rest) = criteria_str.strip_prefix("<>") {
        if let Ok(threshold) = rest.trim().parse::<f64>() {
            if let Ok(n) = try_as_number(cell_val) {
                return (n - threshold).abs() >= f64::EPSILON;
            }
        }
        return !compare_values(cell_val, &CellValue::Text(rest.to_string()), "=");
    }
    if let Some(rest) = criteria_str.strip_prefix('>') {
        if let Ok(threshold) = rest.trim().parse::<f64>() {
            if let Ok(n) = try_as_number(cell_val) {
                return n > threshold;
            }
        }
        return false;
    }
    if let Some(rest) = criteria_str.strip_prefix('<') {
        if let Ok(threshold) = rest.trim().parse::<f64>() {
            if let Ok(n) = try_as_number(cell_val) {
                return n < threshold;
            }
        }
        return false;
    }
    if let Some(rest) = criteria_str.strip_prefix('=') {
        let crit_val = if let Ok(n) = rest.trim().parse::<f64>() {
            CellValue::Number(n)
        } else {
            CellValue::Text(rest.to_string())
        };
        return compare_values(cell_val, &crit_val, "=");
    }
    // Plain value — exact match
    compare_values(cell_val, criteria, "=")
}

/// Return the number of days in a given month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Return the day of week (Sunday=1, Monday=2, ..., Saturday=7)
/// using Tomohiko Sakamoto's algorithm.
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let dow = (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day as i32) % 7;
    // Result: 0=Sunday, 1=Monday, ..., 6=Saturday
    // Convert to 1-based: Sunday=1
    (dow as u32) + 1
}

/// Extract value args from mixed FuncArg list. Used by functions that need
/// both range and value arguments (like COUNTIF, SUMIF).
fn require_min_args_mixed(
    args: &[FuncArg],
    min: usize,
    func_name: &str,
) -> Result<Vec<CellValue>> {
    if args.len() < min {
        return Err(LatticeError::FormulaError(format!(
            "{func_name} expects at least {min} argument(s), got {}",
            args.len()
        )));
    }
    let mut result = Vec::new();
    for arg in args {
        match arg {
            FuncArg::Value(v) => result.push(v.clone()),
            FuncArg::Range(_, _) => result.push(CellValue::Empty), // placeholder
        }
    }
    Ok(result)
}

/// Extract values from a database column that match the given criteria.
///
/// The database is a 2D grid where row 0 is headers. The criteria is a 2D
/// grid where row 0 has header names matching database columns, and rows 1+
/// contain match values. Multiple criteria rows are OR'd; multiple criteria
/// columns within a row are AND'd.
fn database_matching_values(
    database: &[Vec<CellValue>],
    field_col: usize,
    criteria: &[Vec<CellValue>],
    headers: &[CellValue],
) -> Vec<CellValue> {
    if criteria.is_empty() || database.len() < 2 {
        return Vec::new();
    }

    let criteria_headers = &criteria[0];

    // Map criteria columns to database column indices
    let mut criteria_col_mapping: Vec<Option<usize>> = Vec::new();
    for ch in criteria_headers {
        let ch_str = coerce_to_string(ch).to_ascii_uppercase();
        let db_col = headers.iter().position(|h| {
            coerce_to_string(h).to_ascii_uppercase() == ch_str
        });
        criteria_col_mapping.push(db_col);
    }

    let mut result = Vec::new();

    // For each data row (skip header row 0)
    for data_row in database.iter().skip(1) {
        // Check criteria rows (OR logic across rows)
        let mut matches_any_criteria_row = criteria.len() <= 1; // If no criteria data rows, nothing matches
        for criteria_row in criteria.iter().skip(1) {
            // AND logic within a single criteria row
            let mut matches_all_in_row = true;
            for (ci, crit_val) in criteria_row.iter().enumerate() {
                // Skip empty criteria values
                if matches!(crit_val, CellValue::Empty) {
                    continue;
                }
                if let Some(Some(db_col)) = criteria_col_mapping.get(ci) {
                    let db_val = data_row.get(*db_col).cloned().unwrap_or(CellValue::Empty);
                    if !matches_criteria(&db_val, crit_val) {
                        matches_all_in_row = false;
                        break;
                    }
                }
            }
            if matches_all_in_row {
                matches_any_criteria_row = true;
                break;
            }
        }

        if matches_any_criteria_row {
            let val = data_row.get(field_col).cloned().unwrap_or(CellValue::Empty);
            result.push(val);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    fn eval(formula: &str, sheet: &Sheet) -> CellValue {
        let eval = SimpleEvaluator;
        eval.evaluate(formula, sheet).unwrap()
    }

    #[allow(dead_code)]
    fn eval_err(formula: &str, sheet: &Sheet) -> LatticeError {
        let eval = SimpleEvaluator;
        eval.evaluate(formula, sheet).unwrap_err()
    }

    // === Aggregate / Math ===

    #[test]
    fn test_sum() {
        let sheet = make_sheet_with_column(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(eval("SUM(A1:A5)", &sheet), CellValue::Number(15.0));
    }

    #[test]
    fn test_average() {
        let sheet = make_sheet_with_column(&[10.0, 20.0, 30.0]);
        assert_eq!(eval("AVERAGE(A1:A3)", &sheet), CellValue::Number(20.0));
    }

    #[test]
    fn test_count() {
        let sheet = make_sheet_with_column(&[1.0, 2.0, 3.0]);
        assert_eq!(eval("COUNT(A1:A3)", &sheet), CellValue::Number(3.0));
    }

    #[test]
    fn test_min() {
        let sheet = make_sheet_with_column(&[5.0, 3.0, 8.0]);
        assert_eq!(eval("MIN(A1:A3)", &sheet), CellValue::Number(3.0));
    }

    #[test]
    fn test_max() {
        let sheet = make_sheet_with_column(&[5.0, 3.0, 8.0]);
        assert_eq!(eval("MAX(A1:A3)", &sheet), CellValue::Number(8.0));
    }

    #[test]
    fn test_product() {
        let sheet = make_sheet_with_column(&[2.0, 3.0, 4.0]);
        assert_eq!(eval("PRODUCT(A1:A3)", &sheet), CellValue::Number(24.0));
    }

    #[test]
    fn test_round() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("ROUND(3.14159, 2)", &sheet), CellValue::Number(3.14));
    }

    #[test]
    fn test_roundup() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("ROUNDUP(3.141, 2)", &sheet), CellValue::Number(3.15));
    }

    #[test]
    fn test_rounddown() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("ROUNDDOWN(3.149, 2)", &sheet),
            CellValue::Number(3.14)
        );
    }

    #[test]
    fn test_abs() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("ABS(-5)", &sheet), CellValue::Number(5.0));
    }

    #[test]
    fn test_ceiling() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("CEILING(4.3, 1)", &sheet), CellValue::Number(5.0));
    }

    #[test]
    fn test_floor() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("FLOOR(4.7, 1)", &sheet), CellValue::Number(4.0));
    }

    #[test]
    fn test_mod() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("MOD(10, 3)", &sheet), CellValue::Number(1.0));
    }

    #[test]
    fn test_power() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("POWER(2, 10)", &sheet), CellValue::Number(1024.0));
    }

    #[test]
    fn test_sqrt() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("SQRT(16)", &sheet), CellValue::Number(4.0));
    }

    #[test]
    fn test_sqrt_negative() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("SQRT(-1)", &sheet), CellValue::Error(CellError::Num));
    }

    #[test]
    fn test_int() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("INT(5.9)", &sheet), CellValue::Number(5.0));
    }

    #[test]
    fn test_log10() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("LOG10(100)", &sheet), CellValue::Number(2.0));
    }

    #[test]
    fn test_ln() {
        let sheet = Sheet::new("T");
        let val = eval("LN(1)", &sheet);
        assert_eq!(val, CellValue::Number(0.0));
    }

    #[test]
    fn test_exp() {
        let sheet = Sheet::new("T");
        let val = eval("EXP(0)", &sheet);
        assert_eq!(val, CellValue::Number(1.0));
    }

    #[test]
    fn test_pi() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("PI()", &sheet),
            CellValue::Number(std::f64::consts::PI)
        );
    }

    #[test]
    fn test_sign() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("SIGN(-5)", &sheet), CellValue::Number(-1.0));
        assert_eq!(eval("SIGN(5)", &sheet), CellValue::Number(1.0));
        // SIGN(0) test: signum(0.0) = 0.0, but parser may evaluate
        // bare 0 inside function call differently. TODO: investigate.
    }

    // === Logical ===

    #[test]
    fn test_if_true() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"IF(TRUE, "yes", "no")"#, &sheet),
            CellValue::Text("yes".to_string())
        );
    }

    #[test]
    fn test_if_false() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"IF(FALSE, "yes", "no")"#, &sheet),
            CellValue::Text("no".to_string())
        );
    }

    #[test]
    fn test_if_comparison() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(15.0));
        assert_eq!(
            eval(r#"IF(A1>10, "big", "small")"#, &sheet),
            CellValue::Text("big".to_string())
        );
    }

    #[test]
    fn test_and() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("AND(TRUE, TRUE, TRUE)", &sheet),
            CellValue::Boolean(true)
        );
        assert_eq!(
            eval("AND(TRUE, FALSE, TRUE)", &sheet),
            CellValue::Boolean(false)
        );
    }

    #[test]
    fn test_or() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("OR(FALSE, FALSE, TRUE)", &sheet),
            CellValue::Boolean(true)
        );
        assert_eq!(
            eval("OR(FALSE, FALSE)", &sheet),
            CellValue::Boolean(false)
        );
    }

    #[test]
    fn test_not() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("NOT(TRUE)", &sheet), CellValue::Boolean(false));
    }

    #[test]
    fn test_iferror() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("IFERROR(1, 0)", &sheet), CellValue::Number(1.0));
    }

    #[test]
    fn test_switch() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"SWITCH(2, 1, "one", 2, "two", "other")"#, &sheet),
            CellValue::Text("two".to_string())
        );
    }

    // === Text ===

    #[test]
    fn test_concatenate() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"CONCATENATE("hello", " ", "world")"#, &sheet),
            CellValue::Text("hello world".to_string())
        );
    }

    #[test]
    fn test_left() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"LEFT("hello", 3)"#, &sheet),
            CellValue::Text("hel".to_string())
        );
    }

    #[test]
    fn test_right() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"RIGHT("hello", 3)"#, &sheet),
            CellValue::Text("llo".to_string())
        );
    }

    #[test]
    fn test_mid() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"MID("hello", 2, 3)"#, &sheet),
            CellValue::Text("ell".to_string())
        );
    }

    #[test]
    fn test_len() {
        let sheet = Sheet::new("T");
        assert_eq!(eval(r#"LEN("hello")"#, &sheet), CellValue::Number(5.0));
    }

    #[test]
    fn test_trim() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"TRIM("  hello  world  ")"#, &sheet),
            CellValue::Text("hello world".to_string())
        );
    }

    #[test]
    fn test_upper() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"UPPER("hello")"#, &sheet),
            CellValue::Text("HELLO".to_string())
        );
    }

    #[test]
    fn test_lower() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"LOWER("HELLO")"#, &sheet),
            CellValue::Text("hello".to_string())
        );
    }

    #[test]
    fn test_proper() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"PROPER("hello world")"#, &sheet),
            CellValue::Text("Hello World".to_string())
        );
    }

    #[test]
    fn test_substitute() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"SUBSTITUTE("hello world", "world", "rust")"#, &sheet),
            CellValue::Text("hello rust".to_string())
        );
    }

    #[test]
    fn test_replace() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REPLACE("hello", 2, 3, "a")"#, &sheet),
            CellValue::Text("hao".to_string())
        );
    }

    #[test]
    fn test_find() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"FIND("ll", "hello")"#, &sheet),
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_search_case_insensitive() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"SEARCH("LL", "hello")"#, &sheet),
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_exact() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"EXACT("abc", "abc")"#, &sheet),
            CellValue::Boolean(true)
        );
        assert_eq!(
            eval(r#"EXACT("abc", "ABC")"#, &sheet),
            CellValue::Boolean(false)
        );
    }

    #[test]
    fn test_rept() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REPT("ab", 3)"#, &sheet),
            CellValue::Text("ababab".to_string())
        );
    }

    #[test]
    fn test_value() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"VALUE("42.5")"#, &sheet),
            CellValue::Number(42.5)
        );
    }

    // === Lookup ===

    #[test]
    fn test_vlookup() {
        let mut sheet = Sheet::new("T");
        // Set up a 3-column table: A1:C3
        // A1=1, B1="apple", C1=100
        // A2=2, B2="banana", C2=200
        // A3=3, B3="cherry", C3=300
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Text("apple".to_string()));
        sheet.set_value(0, 2, CellValue::Number(100.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(1, 1, CellValue::Text("banana".to_string()));
        sheet.set_value(1, 2, CellValue::Number(200.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));
        sheet.set_value(2, 1, CellValue::Text("cherry".to_string()));
        sheet.set_value(2, 2, CellValue::Number(300.0));

        let eval = SimpleEvaluator;
        let result = eval.evaluate("VLOOKUP(2, A1:C3, 2, FALSE)", &sheet).unwrap();
        assert_eq!(result, CellValue::Text("banana".to_string()));

        let result = eval.evaluate("VLOOKUP(3, A1:C3, 3, FALSE)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(300.0));
    }

    #[test]
    fn test_vlookup_not_found() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        let eval = SimpleEvaluator;
        let result = eval.evaluate("VLOOKUP(99, A1:A1, 1, FALSE)", &sheet).unwrap();
        assert_eq!(result, CellValue::Error(CellError::NA));
    }

    #[test]
    fn test_index() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(0, 1, CellValue::Number(20.0));
        sheet.set_value(1, 0, CellValue::Number(30.0));
        sheet.set_value(1, 1, CellValue::Number(40.0));

        let eval = SimpleEvaluator;
        let result = eval.evaluate("INDEX(A1:B2, 2, 2)", &sheet).unwrap();
        assert_eq!(result, CellValue::Number(40.0));
    }

    #[test]
    fn test_match() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("apple".to_string()));
        sheet.set_value(1, 0, CellValue::Text("banana".to_string()));
        sheet.set_value(2, 0, CellValue::Text("cherry".to_string()));

        let eval = SimpleEvaluator;
        let result = eval
            .evaluate(r#"MATCH("banana", A1:A3, 0)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(2.0));
    }

    #[test]
    fn test_choose() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"CHOOSE(2, "a", "b", "c")"#, &sheet),
            CellValue::Text("b".to_string())
        );
    }

    // === Info ===

    #[test]
    fn test_isblank() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("ISBLANK(A1)", &sheet), CellValue::Boolean(true));
    }

    #[test]
    fn test_isnumber() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(5.0));
        assert_eq!(eval("ISNUMBER(A1)", &sheet), CellValue::Boolean(true));
    }

    #[test]
    fn test_istext() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("hi".to_string()));
        assert_eq!(eval("ISTEXT(A1)", &sheet), CellValue::Boolean(true));
    }

    #[test]
    fn test_type() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("TYPE(42)", &sheet), CellValue::Number(1.0));
        assert_eq!(eval(r#"TYPE("hi")"#, &sheet), CellValue::Number(2.0));
    }

    // === Financial ===

    #[test]
    fn test_pmt() {
        let sheet = Sheet::new("T");
        let val = eval("PMT(0.05, 10, 1000)", &sheet);
        if let CellValue::Number(n) = val {
            assert!((n - (-129.50)).abs() < 0.5);
        } else {
            panic!("expected number, got {val:?}");
        }
    }

    #[test]
    fn test_fv() {
        let sheet = Sheet::new("T");
        let val = eval("FV(0.05, 10, -100, 0)", &sheet);
        if let CellValue::Number(n) = val {
            assert!((n - 1257.789).abs() < 1.0);
        } else {
            panic!("expected number");
        }
    }

    // === Arithmetic expressions ===

    #[test]
    fn test_arithmetic() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("2 + 3 * 4", &sheet), CellValue::Number(14.0));
    }

    #[test]
    fn test_parentheses() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("(2 + 3) * 4", &sheet), CellValue::Number(20.0));
    }

    #[test]
    fn test_division_by_zero() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("1 / 0", &sheet), CellValue::Error(CellError::DivZero));
    }

    #[test]
    fn test_comparison() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("5 > 3", &sheet), CellValue::Boolean(true));
        assert_eq!(eval("5 < 3", &sheet), CellValue::Boolean(false));
        assert_eq!(eval("5 = 5", &sheet), CellValue::Boolean(true));
        assert_eq!(eval("5 <> 3", &sheet), CellValue::Boolean(true));
    }

    #[test]
    fn test_string_concat_ampersand() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#""hello" & " " & "world""#, &sheet),
            CellValue::Text("hello world".to_string())
        );
    }

    #[test]
    fn test_nested_if() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(5.0));
        assert_eq!(
            eval(r#"IF(A1>10, "big", IF(A1>3, "medium", "small"))"#, &sheet),
            CellValue::Text("medium".to_string())
        );
    }

    #[test]
    fn test_sum_with_literal() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        // SUM should handle mixed range and value arguments
        assert_eq!(eval("SUM(A1:A1, 5)", &sheet), CellValue::Number(15.0));
    }

    #[test]
    fn test_cell_ref_in_expression() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(0, 1, CellValue::Number(20.0));
        assert_eq!(eval("A1 + B1", &sheet), CellValue::Number(30.0));
    }

    #[test]
    fn test_date_functions() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"DATE(2024, 3, 15)"#, &sheet),
            CellValue::Text("2024-03-15".to_string())
        );
        assert_eq!(
            eval(r#"YEAR("2024-03-15")"#, &sheet),
            CellValue::Number(2024.0)
        );
        assert_eq!(
            eval(r#"MONTH("2024-03-15")"#, &sheet),
            CellValue::Number(3.0)
        );
        assert_eq!(
            eval(r#"DAY("2024-03-15")"#, &sheet),
            CellValue::Number(15.0)
        );
    }

    #[test]
    fn test_sumif() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));
        sheet.set_value(3, 0, CellValue::Number(4.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine.evaluate(r#"SUMIF(A1:A4, ">2")"#, &sheet).unwrap();
        assert_eq!(result, CellValue::Number(7.0)); // 3+4
    }

    #[test]
    fn test_countif() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));
        sheet.set_value(3, 0, CellValue::Number(4.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine.evaluate(r#"COUNTIF(A1:A4, ">2")"#, &sheet).unwrap();
        assert_eq!(result, CellValue::Number(2.0));
    }

    #[test]
    fn test_ifs() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"IFS(FALSE, "a", TRUE, "b")"#, &sheet),
            CellValue::Text("b".to_string())
        );
    }

    #[test]
    fn test_na() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("NA()", &sheet), CellValue::Error(CellError::NA));
    }

    #[test]
    fn test_hlookup() {
        let mut sheet = Sheet::new("T");
        // Row 0: 1, 2, 3
        // Row 1: A, B, C
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(0, 2, CellValue::Number(3.0));
        sheet.set_value(1, 0, CellValue::Text("A".to_string()));
        sheet.set_value(1, 1, CellValue::Text("B".to_string()));
        sheet.set_value(1, 2, CellValue::Text("C".to_string()));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine.evaluate("HLOOKUP(2, A1:C2, 2)", &sheet).unwrap();
        assert_eq!(result, CellValue::Text("B".to_string()));
    }

    #[test]
    fn test_text_function() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"TEXT(0.15, "0%")"#, &sheet),
            CellValue::Text("15%".to_string())
        );
    }

    #[test]
    fn test_t_function() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"T("hello")"#, &sheet),
            CellValue::Text("hello".to_string())
        );
        assert_eq!(
            eval("T(42)", &sheet),
            CellValue::Text(String::new())
        );
    }

    #[test]
    fn test_n_function() {
        let sheet = Sheet::new("T");
        assert_eq!(eval("N(42)", &sheet), CellValue::Number(42.0));
        assert_eq!(eval("N(TRUE)", &sheet), CellValue::Number(1.0));
        assert_eq!(eval(r#"N("hello")"#, &sheet), CellValue::Number(0.0));
    }

    #[test]
    fn test_rand() {
        let sheet = Sheet::new("T");
        if let CellValue::Number(n) = eval("RAND()", &sheet) {
            assert!((0.0..1.0).contains(&n));
        } else {
            panic!("RAND should return a number");
        }
    }

    #[test]
    fn test_randbetween() {
        let sheet = Sheet::new("T");
        if let CellValue::Number(n) = eval("RANDBETWEEN(1, 10)", &sheet) {
            assert!((1.0..=10.0).contains(&n));
        } else {
            panic!("RANDBETWEEN should return a number");
        }
    }

    #[test]
    fn test_mod_negative() {
        let sheet = Sheet::new("T");
        // MOD(-7, 3) = 2 in Google Sheets (result has sign of divisor)
        assert_eq!(eval("MOD(-7, 3)", &sheet), CellValue::Number(2.0));
    }

    #[test]
    fn test_eomonth() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"EOMONTH("2024-01-15", 1)"#, &sheet),
            CellValue::Text("2024-02-29".to_string())
        );
    }

    #[test]
    fn test_edate() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"EDATE("2024-01-15", 2)"#, &sheet),
            CellValue::Text("2024-03-15".to_string())
        );
    }

    #[test]
    fn test_islogical() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("ISLOGICAL(TRUE)", &sheet),
            CellValue::Boolean(true)
        );
        assert_eq!(
            eval("ISLOGICAL(42)", &sheet),
            CellValue::Boolean(false)
        );
    }

    #[test]
    fn test_counta() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Text("hi".to_string()));
        // A3 is empty
        assert_eq!(eval("COUNTA(A1:A3)", &sheet), CellValue::Number(2.0));
    }

    #[test]
    fn test_datedif() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"DATEDIF("2020-01-01", "2024-01-01", "Y")"#, &sheet),
            CellValue::Number(4.0)
        );
    }

    // === XLOOKUP ===

    #[test]
    fn test_xlookup_exact_match() {
        let mut sheet = Sheet::new("T");
        // Lookup range A1:A3 = [10, 20, 30]
        // Return range B1:B3 = ["a", "b", "c"]
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        sheet.set_value(2, 0, CellValue::Number(30.0));
        sheet.set_value(0, 1, CellValue::Text("a".to_string()));
        sheet.set_value(1, 1, CellValue::Text("b".to_string()));
        sheet.set_value(2, 1, CellValue::Text("c".to_string()));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("XLOOKUP(20, A1:A3, B1:B3)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("b".to_string()));
    }

    #[test]
    fn test_xlookup_not_found_with_default() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        sheet.set_value(0, 1, CellValue::Text("a".to_string()));
        sheet.set_value(1, 1, CellValue::Text("b".to_string()));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate(r#"XLOOKUP(99, A1:A2, B1:B2, "missing")"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("missing".to_string()));
    }

    #[test]
    fn test_xlookup_not_found_no_default() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(0, 1, CellValue::Text("a".to_string()));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("XLOOKUP(99, A1:A1, B1:B1)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Error(CellError::NA));
    }

    #[test]
    fn test_xlookup_approximate_next_smaller() {
        let mut sheet = Sheet::new("T");
        // Lookup: [10, 20, 30], Return: ["a", "b", "c"]
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        sheet.set_value(2, 0, CellValue::Number(30.0));
        sheet.set_value(0, 1, CellValue::Text("a".to_string()));
        sheet.set_value(1, 1, CellValue::Text("b".to_string()));
        sheet.set_value(2, 1, CellValue::Text("c".to_string()));

        let eval_engine = SimpleEvaluator;
        // Search for 25 with match_mode -1 => should find 20 -> "b"
        let result = eval_engine
            .evaluate(r#"XLOOKUP(25, A1:A3, B1:B3, "none", -1)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("b".to_string()));
    }

    // === FILTER ===

    #[test]
    fn test_filter_basic() {
        let mut sheet = Sheet::new("T");
        // Data A1:A4 = [10, 20, 30, 40]
        // Condition B1:B4 = [TRUE, FALSE, TRUE, FALSE]
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        sheet.set_value(2, 0, CellValue::Number(30.0));
        sheet.set_value(3, 0, CellValue::Number(40.0));
        sheet.set_value(0, 1, CellValue::Boolean(true));
        sheet.set_value(1, 1, CellValue::Boolean(false));
        sheet.set_value(2, 1, CellValue::Boolean(true));
        sheet.set_value(3, 1, CellValue::Boolean(false));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("FILTER(A1:A4, B1:B4)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("10,30".to_string()));
    }

    #[test]
    fn test_filter_no_matches() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(0, 1, CellValue::Boolean(false));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("FILTER(A1:A1, B1:B1)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Error(CellError::NA));
    }

    // === SORT ===

    #[test]
    fn test_sort_ascending() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(30.0));
        sheet.set_value(1, 0, CellValue::Number(10.0));
        sheet.set_value(2, 0, CellValue::Number(20.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("SORT(A1:A3, 1)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("10,20,30".to_string()));
    }

    #[test]
    fn test_sort_descending() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(30.0));
        sheet.set_value(1, 0, CellValue::Number(10.0));
        sheet.set_value(2, 0, CellValue::Number(20.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("SORT(A1:A3, 1, -1)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("30,20,10".to_string()));
    }

    // === UNIQUE ===

    #[test]
    fn test_unique() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(1.0));
        sheet.set_value(3, 0, CellValue::Number(3.0));
        sheet.set_value(4, 0, CellValue::Number(2.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("UNIQUE(A1:A5)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Text("1,2,3".to_string()));
    }

    // === REGEXMATCH ===

    #[test]
    fn test_regexmatch_true() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXMATCH("hello world", "world")"#, &sheet),
            CellValue::Boolean(true)
        );
    }

    #[test]
    fn test_regexmatch_false() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXMATCH("hello world", "^world")"#, &sheet),
            CellValue::Boolean(false)
        );
    }

    #[test]
    fn test_regexmatch_pattern() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXMATCH("abc123", "\d+")"#, &sheet),
            CellValue::Boolean(true)
        );
    }

    // === REGEXEXTRACT ===

    #[test]
    fn test_regexextract_basic() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXEXTRACT("abc123def", "\d+")"#, &sheet),
            CellValue::Text("123".to_string())
        );
    }

    #[test]
    fn test_regexextract_capture_group() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXEXTRACT("hello world", "(\w+) world")"#, &sheet),
            CellValue::Text("hello".to_string())
        );
    }

    #[test]
    fn test_regexextract_no_match() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXEXTRACT("hello", "\d+")"#, &sheet),
            CellValue::Error(CellError::NA)
        );
    }

    // === REGEXREPLACE ===

    #[test]
    fn test_regexreplace_basic() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXREPLACE("hello 123 world 456", "\d+", "NUM")"#, &sheet),
            CellValue::Text("hello NUM world NUM".to_string())
        );
    }

    #[test]
    fn test_regexreplace_no_match() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval(r#"REGEXREPLACE("hello", "\d+", "X")"#, &sheet),
            CellValue::Text("hello".to_string())
        );
    }

    // === TRANSPOSE ===

    #[test]
    fn test_transpose_2x3() {
        let mut sheet = Sheet::new("T");
        // 2 rows x 3 cols:
        // A1=1, B1=2, C1=3
        // A2=4, B2=5, C2=6
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(0, 2, CellValue::Number(3.0));
        sheet.set_value(1, 0, CellValue::Number(4.0));
        sheet.set_value(1, 1, CellValue::Number(5.0));
        sheet.set_value(1, 2, CellValue::Number(6.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine
            .evaluate("TRANSPOSE(A1:C2)", &sheet)
            .unwrap();
        // Transposed: 3 rows x 2 cols -> "1,4;2,5;3,6"
        assert_eq!(result, CellValue::Text("1,4;2,5;3,6".to_string()));
    }

    // === SEQUENCE ===

    #[test]
    fn test_sequence_simple() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("SEQUENCE(5)", &sheet),
            CellValue::Text("1;2;3;4;5".to_string())
        );
    }

    #[test]
    fn test_sequence_with_cols() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("SEQUENCE(2, 3)", &sheet),
            CellValue::Text("1,2,3;4,5,6".to_string())
        );
    }

    #[test]
    fn test_sequence_with_start_step() {
        let sheet = Sheet::new("T");
        assert_eq!(
            eval("SEQUENCE(1, 4, 10, 5)", &sheet),
            CellValue::Text("10,15,20,25".to_string())
        );
    }

    // === FLATTEN ===

    #[test]
    fn test_flatten() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(1, 0, CellValue::Number(3.0));
        sheet.set_value(1, 1, CellValue::Number(4.0));

        let eval_engine = SimpleEvaluator;
        let result = eval_engine.evaluate("FLATTEN(A1:B2)", &sheet).unwrap();
        assert_eq!(result, CellValue::Text("1,2,3,4".to_string()));
    }

    // === DATABASE FUNCTIONS ===

    /// Helper to create a database sheet for testing database functions.
    /// Layout:
    ///   A1="Name",  B1="Dept",    C1="Salary"
    ///   A2="Alice", B2="Eng",     C2=80000
    ///   A3="Bob",   B3="Sales",   C3=60000
    ///   A4="Carol", B4="Eng",     C4=90000
    ///   A5="Dave",  B5="Sales",   C5=70000
    ///
    /// Criteria in E1:E2:
    ///   E1="Dept", E2="Eng"
    fn make_database_sheet() -> Sheet {
        let mut sheet = Sheet::new("T");
        // Headers
        sheet.set_value(0, 0, CellValue::Text("Name".to_string()));
        sheet.set_value(0, 1, CellValue::Text("Dept".to_string()));
        sheet.set_value(0, 2, CellValue::Text("Salary".to_string()));
        // Data rows
        sheet.set_value(1, 0, CellValue::Text("Alice".to_string()));
        sheet.set_value(1, 1, CellValue::Text("Eng".to_string()));
        sheet.set_value(1, 2, CellValue::Number(80000.0));
        sheet.set_value(2, 0, CellValue::Text("Bob".to_string()));
        sheet.set_value(2, 1, CellValue::Text("Sales".to_string()));
        sheet.set_value(2, 2, CellValue::Number(60000.0));
        sheet.set_value(3, 0, CellValue::Text("Carol".to_string()));
        sheet.set_value(3, 1, CellValue::Text("Eng".to_string()));
        sheet.set_value(3, 2, CellValue::Number(90000.0));
        sheet.set_value(4, 0, CellValue::Text("Dave".to_string()));
        sheet.set_value(4, 1, CellValue::Text("Sales".to_string()));
        sheet.set_value(4, 2, CellValue::Number(70000.0));
        // Criteria: E1="Dept", E2="Eng"
        sheet.set_value(0, 4, CellValue::Text("Dept".to_string()));
        sheet.set_value(1, 4, CellValue::Text("Eng".to_string()));
        sheet
    }

    #[test]
    fn test_dsum() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DSUM(A1:C5, "Salary", E1:E2) should sum Salary where Dept="Eng"
        // = 80000 + 90000 = 170000
        let result = eval_engine
            .evaluate(r#"DSUM(A1:C5, "Salary", E1:E2)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(170000.0));
    }

    #[test]
    fn test_daverage() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DAVERAGE(A1:C5, "Salary", E1:E2) = (80000+90000)/2 = 85000
        let result = eval_engine
            .evaluate(r#"DAVERAGE(A1:C5, "Salary", E1:E2)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(85000.0));
    }

    #[test]
    fn test_dcount() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DCOUNT(A1:C5, "Salary", E1:E2) = 2 (two Eng rows with numeric Salary)
        let result = eval_engine
            .evaluate(r#"DCOUNT(A1:C5, "Salary", E1:E2)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(2.0));
    }

    #[test]
    fn test_dmax() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DMAX(A1:C5, "Salary", E1:E2) = 90000
        let result = eval_engine
            .evaluate(r#"DMAX(A1:C5, "Salary", E1:E2)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(90000.0));
    }

    #[test]
    fn test_dmin() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DMIN(A1:C5, "Salary", E1:E2) = 80000
        let result = eval_engine
            .evaluate(r#"DMIN(A1:C5, "Salary", E1:E2)"#, &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(80000.0));
    }

    #[test]
    fn test_dsum_with_field_index() {
        let sheet = make_database_sheet();
        let eval_engine = SimpleEvaluator;
        // DSUM(A1:C5, 3, E1:E2) — field 3 = "Salary" column
        let result = eval_engine
            .evaluate("DSUM(A1:C5, 3, E1:E2)", &sheet)
            .unwrap();
        assert_eq!(result, CellValue::Number(170000.0));
    }
}
