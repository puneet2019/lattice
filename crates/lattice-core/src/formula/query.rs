//! QUERY function — query string parser.
//!
//! Parses a simplified Google Visualization API Query Language string into
//! a [`Query`] AST. Supports SELECT, WHERE, ORDER BY, GROUP BY, LIMIT, and
//! LABEL clauses.

use crate::error::{LatticeError, Result};

// ---- AST types -----------------------------------------------------------

/// A parsed QUERY statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub select: Vec<SelectItem>,
    pub where_clause: Option<WhereExpr>,
    pub order_by: Vec<(ColRef, SortOrder)>,
    pub group_by: Vec<ColRef>,
    pub limit: Option<usize>,
    pub labels: Vec<(ColRef, String)>,
}

/// A single item in the SELECT clause.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectItem {
    Column(ColRef),
    Aggregate(AggFunc, ColRef),
}

/// 0-based column index within the data range (A=0, B=1, ...).
pub type ColRef = usize;

/// Aggregation functions for GROUP BY queries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggFunc {
    Sum,
    Count,
    Avg,
    Min,
    Max,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Boolean expression for the WHERE clause.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereExpr {
    Comparison(ColRef, CompOp, Literal),
    IsNull(ColRef),
    IsNotNull(ColRef),
    And(Box<WhereExpr>, Box<WhereExpr>),
    Or(Box<WhereExpr>, Box<WhereExpr>),
}

/// Comparison operators for WHERE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompOp {
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
}

/// A literal value in a WHERE comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    Text(String),
    Boolean(bool),
}

// ---- Parser entry point --------------------------------------------------

/// Parse a Google-Sheets-style query string into a [`Query`] AST.
pub fn parse_query(input: &str) -> Result<Query> {
    let tokens = tokenize_query(input)?;
    let mut pos = 0;
    let mut q = Query {
        select: Vec::new(),
        where_clause: None,
        order_by: Vec::new(),
        group_by: Vec::new(),
        limit: None,
        labels: Vec::new(),
    };
    while pos < tokens.len() {
        match tokens[pos].to_ascii_uppercase().as_str() {
            "SELECT" => {
                pos += 1;
                q.select = parse_select(&tokens, &mut pos)?;
            }
            "WHERE" => {
                pos += 1;
                q.where_clause = Some(parse_where(&tokens, &mut pos)?);
            }
            "ORDER" => {
                expect_next(&tokens, pos + 1, "BY", "ORDER")?;
                pos += 2;
                q.order_by = parse_order_by(&tokens, &mut pos)?;
            }
            "GROUP" => {
                expect_next(&tokens, pos + 1, "BY", "GROUP")?;
                pos += 2;
                q.group_by = parse_group_by(&tokens, &mut pos)?;
            }
            "LIMIT" => {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(qerr("expected number after LIMIT"));
                }
                q.limit = Some(parse_usize(&tokens[pos])?);
                pos += 1;
            }
            "LABEL" => {
                pos += 1;
                q.labels = parse_labels(&tokens, &mut pos)?;
            }
            other => return Err(qerr(&format!("unexpected keyword '{other}'"))),
        }
    }
    Ok(q)
}

// ---- Tokenizer -----------------------------------------------------------

fn tokenize_query(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '\'' || ch == '"' {
            let quote = ch;
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    Some(c) if c == quote => break,
                    Some(c) => s.push(c),
                    None => return Err(qerr("unterminated string literal")),
                }
            }
            tokens.push(format!("'{s}'"));
            continue;
        }
        if ch == ',' || ch == '(' || ch == ')' {
            tokens.push(ch.to_string());
            chars.next();
            continue;
        }
        if matches!(ch, '>' | '<' | '=' | '!') {
            chars.next();
            if let Some(&nx) = chars.peek() {
                let is_two_char =
                    matches!((ch, nx), ('>', '=') | ('<', '=') | ('<', '>') | ('!', '='));
                if is_two_char {
                    tokens.push(format!("{ch}{nx}"));
                    chars.next();
                    continue;
                }
            }
            tokens.push(ch.to_string());
            continue;
        }
        if ch.is_ascii_digit()
            || (ch == '-' && chars.clone().nth(1).is_some_and(|c| c.is_ascii_digit()))
        {
            let mut n = String::new();
            if ch == '-' {
                n.push('-');
                chars.next();
            }
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    n.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(n);
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' || ch == '*' {
            let mut w = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '*' {
                    w.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(w);
            continue;
        }
        chars.next();
    }
    Ok(tokens)
}

// ---- Clause parsers ------------------------------------------------------

fn parse_select(tokens: &[String], pos: &mut usize) -> Result<Vec<SelectItem>> {
    if *pos < tokens.len() && tokens[*pos] == "*" {
        *pos += 1;
        return Ok(vec![]);
    }
    let mut items = Vec::new();
    loop {
        if *pos >= tokens.len() || is_keyword(&tokens[*pos]) {
            break;
        }
        items.push(parse_select_item(tokens, pos)?);
        if *pos < tokens.len() && tokens[*pos] == "," {
            *pos += 1;
        } else {
            break;
        }
    }
    if items.is_empty() {
        return Err(qerr("SELECT requires at least one column"));
    }
    Ok(items)
}

fn parse_select_item(tokens: &[String], pos: &mut usize) -> Result<SelectItem> {
    if *pos >= tokens.len() {
        return Err(qerr("unexpected end of SELECT"));
    }
    let upper = tokens[*pos].to_ascii_uppercase();
    if let Some(agg) = parse_agg_name(&upper) {
        let has_parens =
            *pos + 3 < tokens.len() && tokens[*pos + 1] == "(" && tokens[*pos + 3] == ")";
        if has_parens {
            let col = parse_col_ref(&tokens[*pos + 2])?;
            *pos += 4;
            return Ok(SelectItem::Aggregate(agg, col));
        }
    }
    let col = parse_col_ref(&tokens[*pos])?;
    *pos += 1;
    Ok(SelectItem::Column(col))
}

fn parse_where(tokens: &[String], pos: &mut usize) -> Result<WhereExpr> {
    let left = parse_where_atom(tokens, pos)?;
    parse_where_logic(tokens, pos, left)
}

fn parse_where_logic(tokens: &[String], pos: &mut usize, left: WhereExpr) -> Result<WhereExpr> {
    if *pos >= tokens.len() {
        return Ok(left);
    }
    match tokens[*pos].to_ascii_uppercase().as_str() {
        "AND" => {
            *pos += 1;
            let r = parse_where_atom(tokens, pos)?;
            parse_where_logic(tokens, pos, WhereExpr::And(Box::new(left), Box::new(r)))
        }
        "OR" => {
            *pos += 1;
            let r = parse_where_atom(tokens, pos)?;
            parse_where_logic(tokens, pos, WhereExpr::Or(Box::new(left), Box::new(r)))
        }
        _ => Ok(left),
    }
}

fn parse_where_atom(tokens: &[String], pos: &mut usize) -> Result<WhereExpr> {
    if *pos >= tokens.len() {
        return Err(qerr("unexpected end of WHERE clause"));
    }
    let col = parse_col_ref(&tokens[*pos])?;
    *pos += 1;
    if *pos >= tokens.len() {
        return Err(qerr("WHERE clause missing operator"));
    }
    if tokens[*pos].eq_ignore_ascii_case("IS") {
        *pos += 1;
        if *pos >= tokens.len() {
            return Err(qerr("expected NULL or NOT after IS"));
        }
        if tokens[*pos].eq_ignore_ascii_case("NOT") {
            *pos += 1;
            if *pos >= tokens.len() || !tokens[*pos].eq_ignore_ascii_case("NULL") {
                return Err(qerr("expected NULL after IS NOT"));
            }
            *pos += 1;
            return Ok(WhereExpr::IsNotNull(col));
        } else if tokens[*pos].eq_ignore_ascii_case("NULL") {
            *pos += 1;
            return Ok(WhereExpr::IsNull(col));
        }
        return Err(qerr("expected NULL or NOT after IS"));
    }
    let op = parse_comp_op(&tokens[*pos])?;
    *pos += 1;
    if *pos >= tokens.len() {
        return Err(qerr("WHERE missing value after operator"));
    }
    let lit = parse_literal(&tokens[*pos])?;
    *pos += 1;
    Ok(WhereExpr::Comparison(col, op, lit))
}

fn parse_order_by(tokens: &[String], pos: &mut usize) -> Result<Vec<(ColRef, SortOrder)>> {
    let mut specs = Vec::new();
    loop {
        if *pos >= tokens.len() || is_keyword(&tokens[*pos]) {
            break;
        }
        let col = parse_col_ref(&tokens[*pos])?;
        *pos += 1;
        let ord = if *pos < tokens.len() {
            match tokens[*pos].to_ascii_uppercase().as_str() {
                "ASC" => {
                    *pos += 1;
                    SortOrder::Asc
                }
                "DESC" => {
                    *pos += 1;
                    SortOrder::Desc
                }
                _ => SortOrder::Asc,
            }
        } else {
            SortOrder::Asc
        };
        specs.push((col, ord));
        if *pos < tokens.len() && tokens[*pos] == "," {
            *pos += 1;
        } else {
            break;
        }
    }
    Ok(specs)
}

fn parse_group_by(tokens: &[String], pos: &mut usize) -> Result<Vec<ColRef>> {
    let mut cols = Vec::new();
    loop {
        if *pos >= tokens.len() || is_keyword(&tokens[*pos]) {
            break;
        }
        cols.push(parse_col_ref(&tokens[*pos])?);
        *pos += 1;
        if *pos < tokens.len() && tokens[*pos] == "," {
            *pos += 1;
        } else {
            break;
        }
    }
    Ok(cols)
}

fn parse_labels(tokens: &[String], pos: &mut usize) -> Result<Vec<(ColRef, String)>> {
    let mut labels = Vec::new();
    loop {
        if *pos >= tokens.len() || is_keyword(&tokens[*pos]) {
            break;
        }
        let col = parse_col_ref(&tokens[*pos])?;
        *pos += 1;
        if *pos >= tokens.len() {
            return Err(qerr("LABEL missing label string"));
        }
        let lbl = parse_string_literal(&tokens[*pos])?;
        *pos += 1;
        labels.push((col, lbl));
        if *pos < tokens.len() && tokens[*pos] == "," {
            *pos += 1;
        } else {
            break;
        }
    }
    Ok(labels)
}

// ---- Small helpers -------------------------------------------------------

fn parse_col_ref(token: &str) -> Result<ColRef> {
    let upper = token.to_ascii_uppercase();
    if upper.is_empty() || !upper.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(qerr(&format!("invalid column reference '{token}'")));
    }
    let mut idx: usize = 0;
    for ch in upper.chars() {
        idx = idx * 26 + (ch as usize - 'A' as usize + 1);
    }
    Ok(idx - 1)
}

fn parse_agg_name(upper: &str) -> Option<AggFunc> {
    match upper {
        "SUM" => Some(AggFunc::Sum),
        "COUNT" => Some(AggFunc::Count),
        "AVG" => Some(AggFunc::Avg),
        "MIN" => Some(AggFunc::Min),
        "MAX" => Some(AggFunc::Max),
        _ => None,
    }
}

fn parse_comp_op(token: &str) -> Result<CompOp> {
    match token {
        "=" => Ok(CompOp::Eq),
        "!=" | "<>" => Ok(CompOp::Neq),
        ">" => Ok(CompOp::Gt),
        "<" => Ok(CompOp::Lt),
        ">=" => Ok(CompOp::Gte),
        "<=" => Ok(CompOp::Lte),
        _ => Err(qerr(&format!("unknown comparison operator '{token}'"))),
    }
}

fn parse_literal(token: &str) -> Result<Literal> {
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 2 {
        return Ok(Literal::Text(token[1..token.len() - 1].to_string()));
    }
    match token.to_ascii_uppercase().as_str() {
        "TRUE" => return Ok(Literal::Boolean(true)),
        "FALSE" => return Ok(Literal::Boolean(false)),
        _ => {}
    }
    token
        .parse::<f64>()
        .map(Literal::Number)
        .map_err(|_| qerr(&format!("invalid literal '{token}'")))
}

fn parse_string_literal(token: &str) -> Result<String> {
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 2 {
        Ok(token[1..token.len() - 1].to_string())
    } else {
        Err(qerr(&format!("expected quoted string, got '{token}'")))
    }
}

fn parse_usize(token: &str) -> Result<usize> {
    token
        .parse::<usize>()
        .map_err(|_| qerr(&format!("expected integer, got '{token}'")))
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token.to_ascii_uppercase().as_str(),
        "SELECT" | "WHERE" | "ORDER" | "GROUP" | "LIMIT" | "LABEL"
    )
}

fn expect_next(tokens: &[String], idx: usize, expected: &str, after: &str) -> Result<()> {
    if idx < tokens.len() && tokens[idx].to_ascii_uppercase() == expected {
        Ok(())
    } else {
        Err(qerr(&format!("expected {expected} after {after}")))
    }
}

/// Shorthand for producing a QUERY-prefixed formula error.
fn qerr(msg: &str) -> LatticeError {
    LatticeError::FormulaError(format!("QUERY: {msg}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_select_star() {
        let q = parse_query("SELECT *").unwrap();
        assert!(q.select.is_empty());
        assert!(q.where_clause.is_none());
    }

    #[test]
    fn parse_select_columns() {
        let q = parse_query("SELECT A, B, C").unwrap();
        assert_eq!(
            q.select,
            vec![
                SelectItem::Column(0),
                SelectItem::Column(1),
                SelectItem::Column(2),
            ]
        );
    }

    #[test]
    fn parse_where_number() {
        let q = parse_query("SELECT A WHERE B > 100").unwrap();
        assert_eq!(
            q.where_clause,
            Some(WhereExpr::Comparison(1, CompOp::Gt, Literal::Number(100.0)))
        );
    }

    #[test]
    fn parse_where_string() {
        let q = parse_query("SELECT A WHERE B = 'Sales'").unwrap();
        assert_eq!(
            q.where_clause,
            Some(WhereExpr::Comparison(
                1,
                CompOp::Eq,
                Literal::Text("Sales".into())
            ))
        );
    }

    #[test]
    fn parse_where_is_null() {
        let q = parse_query("SELECT A WHERE C IS NULL").unwrap();
        assert_eq!(q.where_clause, Some(WhereExpr::IsNull(2)));
    }

    #[test]
    fn parse_where_is_not_null() {
        let q = parse_query("SELECT A WHERE C IS NOT NULL").unwrap();
        assert_eq!(q.where_clause, Some(WhereExpr::IsNotNull(2)));
    }

    #[test]
    fn parse_where_and() {
        let q = parse_query("SELECT A WHERE B > 10 AND C < 50").unwrap();
        assert_eq!(
            q.where_clause,
            Some(WhereExpr::And(
                Box::new(WhereExpr::Comparison(1, CompOp::Gt, Literal::Number(10.0))),
                Box::new(WhereExpr::Comparison(2, CompOp::Lt, Literal::Number(50.0))),
            ))
        );
    }

    #[test]
    fn parse_where_or() {
        let q = parse_query("SELECT A WHERE B = 1 OR B = 2").unwrap();
        assert!(matches!(q.where_clause, Some(WhereExpr::Or(_, _))));
    }

    #[test]
    fn parse_order_by_desc() {
        let q = parse_query("SELECT A ORDER BY A DESC").unwrap();
        assert_eq!(q.order_by, vec![(0, SortOrder::Desc)]);
    }

    #[test]
    fn parse_order_by_default_asc() {
        let q = parse_query("SELECT A ORDER BY B").unwrap();
        assert_eq!(q.order_by, vec![(1, SortOrder::Asc)]);
    }

    #[test]
    fn parse_group_by_with_agg() {
        let q = parse_query("SELECT A, SUM(B) GROUP BY A").unwrap();
        assert_eq!(
            q.select,
            vec![
                SelectItem::Column(0),
                SelectItem::Aggregate(AggFunc::Sum, 1),
            ]
        );
        assert_eq!(q.group_by, vec![0]);
    }

    #[test]
    fn parse_limit() {
        let q = parse_query("SELECT * LIMIT 10").unwrap();
        assert_eq!(q.limit, Some(10));
    }

    #[test]
    fn parse_label() {
        let q = parse_query("SELECT A, B LABEL A 'Name', B 'Total'").unwrap();
        assert_eq!(q.labels, vec![(0, "Name".into()), (1, "Total".into())]);
    }

    #[test]
    fn parse_combined() {
        let q = parse_query("SELECT A, B WHERE B > 50 ORDER BY B DESC LIMIT 5").unwrap();
        assert_eq!(q.select.len(), 2);
        assert!(q.where_clause.is_some());
        assert_eq!(q.order_by, vec![(1, SortOrder::Desc)]);
        assert_eq!(q.limit, Some(5));
    }

    #[test]
    fn parse_col_ref_single_and_double() {
        assert_eq!(parse_col_ref("A").unwrap(), 0);
        assert_eq!(parse_col_ref("Z").unwrap(), 25);
        assert_eq!(parse_col_ref("AA").unwrap(), 26);
    }

    #[test]
    fn parse_error_unterminated_string() {
        assert!(parse_query("SELECT A WHERE B = 'bad").is_err());
    }

    #[test]
    fn parse_error_missing_limit() {
        assert!(parse_query("SELECT A LIMIT").is_err());
    }

    #[test]
    fn parse_multiple_aggregates() {
        let q =
            parse_query("SELECT A, SUM(B), AVG(C), COUNT(D), MIN(E), MAX(F) GROUP BY A").unwrap();
        assert_eq!(q.select.len(), 6);
        assert_eq!(q.select[2], SelectItem::Aggregate(AggFunc::Avg, 2));
        assert_eq!(q.select[4], SelectItem::Aggregate(AggFunc::Min, 4));
    }

    #[test]
    fn parse_neq_operator() {
        let q = parse_query("SELECT A WHERE B <> 0").unwrap();
        assert_eq!(
            q.where_clause,
            Some(WhereExpr::Comparison(1, CompOp::Neq, Literal::Number(0.0)))
        );
    }

    #[test]
    fn parse_boolean_literal() {
        let q = parse_query("SELECT A WHERE B = TRUE").unwrap();
        assert_eq!(
            q.where_clause,
            Some(WhereExpr::Comparison(1, CompOp::Eq, Literal::Boolean(true)))
        );
    }
}
