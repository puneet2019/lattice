//! QUERY executor — runs a parsed [`Query`] against a 2-D data grid.
//!
//! The execution pipeline is:
//! 1. WHERE — filter rows
//! 2. GROUP BY — group and aggregate
//! 3. ORDER BY — sort result rows
//! 4. SELECT — project columns
//! 5. LIMIT — cap row count
//! 6. Prepend header row (with LABEL overrides)

use crate::cell::CellValue;
use crate::error::Result;

use super::query::{
    AggFunc, ColRef, CompOp, Literal, Query, SelectItem, SortOrder, WhereExpr,
};

/// Execute a parsed [`Query`] against a 2-D data grid.
///
/// `data` is row-major: `data[row][col]` is a [`CellValue`].
/// `headers` is the number of leading rows treated as column headers
/// (default 1 in Google Sheets; 0 means no headers).
///
/// Returns `CellValue::Array` with the query result.
pub fn execute_query(data: &[Vec<CellValue>], query: &Query, headers: usize) -> Result<CellValue> {
    if data.is_empty() {
        return Ok(CellValue::Array(vec![]));
    }
    let num_cols = data[0].len();
    let header_rows = if headers <= data.len() { &data[..headers] } else { data };
    let body = if headers < data.len() { &data[headers..] } else { &[] as &[Vec<CellValue>] };

    // 1. WHERE filter
    let filtered: Vec<&Vec<CellValue>> = body
        .iter()
        .filter(|row| match &query.where_clause {
            Some(expr) => eval_where(expr, row, num_cols),
            None => true,
        })
        .collect();

    // 2. GROUP BY + aggregation
    let grouped = if !query.group_by.is_empty() {
        apply_group_by(&filtered, &query.select, &query.group_by)
    } else {
        filtered.iter().map(|r| (*r).clone()).collect()
    };

    // 3. ORDER BY (apply in reverse so first spec is primary)
    let mut sorted = grouped;
    for &(col, ref ord) in query.order_by.iter().rev() {
        sorted.sort_by(|a, b| {
            let va = a.get(col).unwrap_or(&CellValue::Empty);
            let vb = b.get(col).unwrap_or(&CellValue::Empty);
            let cmp = cmp_cell_values(va, vb);
            if *ord == SortOrder::Desc { cmp.reverse() } else { cmp }
        });
    }

    // 4. SELECT projection
    let projected = if query.select.is_empty() {
        sorted // SELECT * keeps all columns
    } else {
        sorted
            .iter()
            .map(|row| {
                query.select.iter().map(|item| {
                    let c = match item {
                        SelectItem::Column(c) | SelectItem::Aggregate(_, c) => *c,
                    };
                    row.get(c).cloned().unwrap_or(CellValue::Empty)
                }).collect()
            })
            .collect()
    };

    // 5. LIMIT
    let limited: Vec<Vec<CellValue>> = match query.limit {
        Some(n) => projected.into_iter().take(n).collect(),
        None => projected,
    };

    // 6. Build result with optional header row
    let mut result = Vec::new();
    if headers > 0 {
        result.push(build_header(header_rows, &query.select, &query.labels, num_cols));
    }
    result.extend(limited);
    Ok(CellValue::Array(result))
}

// ---- WHERE evaluation ----------------------------------------------------

fn eval_where(expr: &WhereExpr, row: &[CellValue], ncols: usize) -> bool {
    match expr {
        WhereExpr::Comparison(col, op, lit) => {
            let val = if *col < ncols { &row[*col] } else { &CellValue::Empty };
            compare_cell_literal(val, op, lit)
        }
        WhereExpr::IsNull(col) => {
            *col >= ncols || matches!(row.get(*col), Some(CellValue::Empty) | None)
        }
        WhereExpr::IsNotNull(col) => {
            *col < ncols && !matches!(row.get(*col), Some(CellValue::Empty) | None)
        }
        WhereExpr::And(a, b) => eval_where(a, row, ncols) && eval_where(b, row, ncols),
        WhereExpr::Or(a, b) => eval_where(a, row, ncols) || eval_where(b, row, ncols),
    }
}

fn compare_cell_literal(val: &CellValue, op: &CompOp, lit: &Literal) -> bool {
    match lit {
        Literal::Number(n) => {
            let Some(v) = cell_to_f64(val) else { return false };
            match op {
                CompOp::Eq => (v - n).abs() < f64::EPSILON,
                CompOp::Neq => (v - n).abs() >= f64::EPSILON,
                CompOp::Gt => v > *n,
                CompOp::Lt => v < *n,
                CompOp::Gte => v >= *n,
                CompOp::Lte => v <= *n,
            }
        }
        Literal::Text(t) => {
            let s = cell_to_string(val).to_ascii_uppercase();
            let t = t.to_ascii_uppercase();
            match op {
                CompOp::Eq => s == t,
                CompOp::Neq => s != t,
                CompOp::Gt => s > t,
                CompOp::Lt => s < t,
                CompOp::Gte => s >= t,
                CompOp::Lte => s <= t,
            }
        }
        Literal::Boolean(b) => {
            let v = match val {
                CellValue::Boolean(x) => *x,
                CellValue::Number(n) => *n != 0.0,
                _ => return false,
            };
            match op {
                CompOp::Eq => v == *b,
                CompOp::Neq => v != *b,
                _ => false,
            }
        }
    }
}

// ---- GROUP BY + aggregation ----------------------------------------------

fn apply_group_by(
    rows: &[&Vec<CellValue>],
    select: &[SelectItem],
    group_cols: &[ColRef],
) -> Vec<Vec<CellValue>> {
    let mut groups: Vec<(Vec<String>, Vec<&Vec<CellValue>>)> = Vec::new();
    for row in rows {
        let key: Vec<String> = group_cols
            .iter()
            .map(|&c| row.get(c).map(cell_to_string).unwrap_or_default())
            .collect();
        if let Some(g) = groups.iter_mut().find(|(k, _)| *k == key) {
            g.1.push(row);
        } else {
            groups.push((key, vec![row]));
        }
    }
    groups
        .iter()
        .map(|(_, grp)| {
            select
                .iter()
                .map(|item| match item {
                    SelectItem::Column(c) => {
                        grp[0].get(*c).cloned().unwrap_or(CellValue::Empty)
                    }
                    SelectItem::Aggregate(func, c) => {
                        let vals: Vec<f64> = grp
                            .iter()
                            .filter_map(|r| r.get(*c).and_then(cell_to_f64))
                            .collect();
                        CellValue::Number(compute_agg(func, &vals))
                    }
                })
                .collect()
        })
        .collect()
}

fn compute_agg(func: &AggFunc, vals: &[f64]) -> f64 {
    match func {
        AggFunc::Sum => vals.iter().sum(),
        AggFunc::Count => vals.len() as f64,
        AggFunc::Avg => {
            if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }
        }
        AggFunc::Min => vals.iter().cloned().fold(f64::INFINITY, f64::min),
        AggFunc::Max => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    }
}

// ---- Header row ----------------------------------------------------------

fn build_header(
    header_rows: &[Vec<CellValue>],
    select: &[SelectItem],
    labels: &[(ColRef, String)],
    num_cols: usize,
) -> Vec<CellValue> {
    let first = header_rows.first();
    if select.is_empty() {
        let mut hdr: Vec<CellValue> = (0..num_cols)
            .map(|c| first.and_then(|r| r.get(c)).cloned().unwrap_or(CellValue::Empty))
            .collect();
        for (col, lbl) in labels {
            if *col < hdr.len() {
                hdr[*col] = CellValue::Text(lbl.clone());
            }
        }
        hdr
    } else {
        select
            .iter()
            .map(|item| {
                let col = match item {
                    SelectItem::Column(c) | SelectItem::Aggregate(_, c) => *c,
                };
                if let Some((_, lbl)) = labels.iter().find(|(c, _)| *c == col) {
                    CellValue::Text(lbl.clone())
                } else {
                    first.and_then(|r| r.get(col)).cloned().unwrap_or(CellValue::Empty)
                }
            })
            .collect()
    }
}

// ---- Cell value helpers --------------------------------------------------

fn cell_to_f64(val: &CellValue) -> Option<f64> {
    match val {
        CellValue::Number(n) => Some(*n),
        CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        CellValue::Text(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn cell_to_string(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => {
            if *n == n.floor() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Boolean(b) | CellValue::Checkbox(b) => {
            if *b { "TRUE".into() } else { "FALSE".into() }
        }
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".into(),
    }
}

fn cmp_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    match (cell_to_f64(a), cell_to_f64(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => cell_to_string(a).cmp(&cell_to_string(b)),
    }
}
