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

use super::query::{AggFunc, ColRef, CompOp, Literal, Query, SelectItem, SortOrder, WhereExpr};

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
    let header_rows = if headers <= data.len() {
        &data[..headers]
    } else {
        data
    };
    let body = if headers < data.len() {
        &data[headers..]
    } else {
        &[] as &[Vec<CellValue>]
    };

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
    // When GROUP BY was applied, columns are already projected to SELECT
    // order, so we remap ORDER BY column refs to their position in SELECT.
    let has_group = !query.group_by.is_empty();
    let mut sorted = grouped;
    for &(col, ref ord) in query.order_by.iter().rev() {
        let effective_col = if has_group {
            remap_col_to_select(col, &query.select)
        } else {
            col
        };
        sorted.sort_by(|a, b| {
            let va = a.get(effective_col).unwrap_or(&CellValue::Empty);
            let vb = b.get(effective_col).unwrap_or(&CellValue::Empty);
            let cmp = cmp_cell_values(va, vb);
            if *ord == SortOrder::Desc {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    // 4. SELECT projection (skip when GROUP BY already projected)
    let projected = if has_group || query.select.is_empty() {
        sorted
    } else {
        sorted
            .iter()
            .map(|row| {
                query
                    .select
                    .iter()
                    .map(|item| {
                        let c = match item {
                            SelectItem::Column(c) | SelectItem::Aggregate(_, c) => *c,
                        };
                        row.get(c).cloned().unwrap_or(CellValue::Empty)
                    })
                    .collect()
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
        result.push(build_header(
            header_rows,
            &query.select,
            &query.labels,
            num_cols,
        ));
    }
    result.extend(limited);
    Ok(CellValue::Array(result))
}

// ---- WHERE evaluation ----------------------------------------------------

fn eval_where(expr: &WhereExpr, row: &[CellValue], ncols: usize) -> bool {
    match expr {
        WhereExpr::Comparison(col, op, lit) => {
            let val = if *col < ncols {
                &row[*col]
            } else {
                &CellValue::Empty
            };
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
            let Some(v) = cell_to_f64(val) else {
                return false;
            };
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
                    SelectItem::Column(c) => grp[0].get(*c).cloned().unwrap_or(CellValue::Empty),
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
            if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
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
            .map(|c| {
                first
                    .and_then(|r| r.get(c))
                    .cloned()
                    .unwrap_or(CellValue::Empty)
            })
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
                    first
                        .and_then(|r| r.get(col))
                        .cloned()
                        .unwrap_or(CellValue::Empty)
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
            if *b {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".into(),
    }
}

/// Find the position of an original column ref within the SELECT list.
/// Falls back to the original index if not found (shouldn't happen in
/// well-formed queries).
fn remap_col_to_select(orig_col: ColRef, select: &[SelectItem]) -> usize {
    select
        .iter()
        .position(|item| {
            let c = match item {
                SelectItem::Column(c) | SelectItem::Aggregate(_, c) => *c,
            };
            c == orig_col
        })
        .unwrap_or(orig_col)
}

fn cmp_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    match (cell_to_f64(a), cell_to_f64(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => cell_to_string(a).cmp(&cell_to_string(b)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::query::parse_query;

    /// Helper to build a test dataset with 1 header row and data rows.
    fn sample_data() -> Vec<Vec<CellValue>> {
        vec![
            // Header
            vec![t("Name"), t("Dept"), n(0.0)], // col C header as number (unusual but valid)
            // Data rows
            vec![t("Alice"), t("Sales"), n(150.0)],
            vec![t("Bob"), t("Eng"), n(200.0)],
            vec![t("Carol"), t("Sales"), n(50.0)],
            vec![t("Dave"), t("Eng"), n(300.0)],
            vec![t("Eve"), t("Sales"), n(100.0)],
        ]
    }

    fn t(s: &str) -> CellValue {
        CellValue::Text(s.into())
    }
    fn n(v: f64) -> CellValue {
        CellValue::Number(v)
    }

    fn unwrap_array(v: CellValue) -> Vec<Vec<CellValue>> {
        match v {
            CellValue::Array(a) => a,
            other => panic!("expected Array, got {other:?}"),
        }
    }

    #[test]
    fn exec_select_star() {
        let data = sample_data();
        let q = parse_query("SELECT *").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + 5 data rows
        assert_eq!(result.len(), 6);
        assert_eq!(result[0][0], t("Name")); // header preserved
    }

    #[test]
    fn exec_select_columns() {
        let data = sample_data();
        let q = parse_query("SELECT A, C").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        assert_eq!(result.len(), 6); // 1 header + 5 data
        // Each row should have 2 columns
        assert_eq!(result[1].len(), 2);
        assert_eq!(result[1][0], t("Alice"));
        assert_eq!(result[1][1], n(150.0));
    }

    #[test]
    fn exec_where_filter() {
        let data = sample_data();
        let q = parse_query("SELECT A WHERE C > 100").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + rows where C > 100 (Alice=150, Bob=200, Dave=300)
        assert_eq!(result.len(), 4);
        assert_eq!(result[1][0], t("Alice"));
        assert_eq!(result[2][0], t("Bob"));
        assert_eq!(result[3][0], t("Dave"));
    }

    #[test]
    fn exec_where_string() {
        let data = sample_data();
        let q = parse_query("SELECT A WHERE B = 'Eng'").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        assert_eq!(result.len(), 3); // header + Bob + Dave
    }

    #[test]
    fn exec_order_by_desc() {
        let data = sample_data();
        let q = parse_query("SELECT A, C ORDER BY C DESC").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // First data row should be Dave (300)
        assert_eq!(result[1][0], t("Dave"));
        assert_eq!(result[1][1], n(300.0));
    }

    #[test]
    fn exec_limit() {
        let data = sample_data();
        let q = parse_query("SELECT * LIMIT 3").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + 3 data rows
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn exec_group_by_sum() {
        let data = sample_data();
        let q = parse_query("SELECT A, SUM(C) GROUP BY A").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + 5 unique names (each person is unique)
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn exec_group_by_department() {
        let data = sample_data();
        let q = parse_query("SELECT B, SUM(C) GROUP BY B").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + 2 departments (Sales, Eng)
        assert_eq!(result.len(), 3);
        // Find Sales group: 150 + 50 + 100 = 300
        let sales_row = result.iter().skip(1).find(|r| r[0] == t("Sales")).unwrap();
        assert_eq!(sales_row[1], n(300.0));
        // Find Eng group: 200 + 300 = 500
        let eng_row = result.iter().skip(1).find(|r| r[0] == t("Eng")).unwrap();
        assert_eq!(eng_row[1], n(500.0));
    }

    #[test]
    fn exec_combined_where_order_limit() {
        let data = sample_data();
        let q = parse_query("SELECT A, C WHERE C > 50 ORDER BY C DESC LIMIT 2").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + 2 rows (Dave=300, Bob=200)
        assert_eq!(result.len(), 3);
        assert_eq!(result[1][0], t("Dave"));
        assert_eq!(result[2][0], t("Bob"));
    }

    #[test]
    fn exec_label_override() {
        let data = sample_data();
        let q = parse_query("SELECT A, C LABEL A 'Employee', C 'Score'").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        assert_eq!(result[0][0], t("Employee"));
        assert_eq!(result[0][1], t("Score"));
    }

    #[test]
    fn exec_no_headers() {
        let data = sample_data();
        let q = parse_query("SELECT *").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 0).unwrap());
        // No header row — all 6 rows are treated as data
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn exec_empty_data() {
        let data: Vec<Vec<CellValue>> = vec![];
        let q = parse_query("SELECT *").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        assert!(result.is_empty());
    }

    #[test]
    fn exec_where_is_not_null() {
        let data = vec![
            vec![t("Name"), t("Value")],
            vec![t("A"), n(10.0)],
            vec![t("B"), CellValue::Empty],
            vec![t("C"), n(30.0)],
        ];
        let q = parse_query("SELECT A WHERE B IS NOT NULL").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        // Header + A + C (B has empty value)
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn exec_avg_aggregation() {
        let data = sample_data();
        let q = parse_query("SELECT B, AVG(C) GROUP BY B").unwrap();
        let result = unwrap_array(execute_query(&data, &q, 1).unwrap());
        let sales_row = result.iter().skip(1).find(|r| r[0] == t("Sales")).unwrap();
        // Sales avg: (150 + 50 + 100) / 3 = 100
        assert_eq!(sales_row[1], n(100.0));
    }
}
