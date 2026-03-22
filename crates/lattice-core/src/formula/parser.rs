/// Tokens produced by the formula tokeniser.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A numeric literal.
    Number(f64),
    /// A cell reference such as `A1` or `$A$1`.
    CellRef(String),
    /// A cross-sheet cell reference such as `Sheet2!A1`.
    /// Contains (sheet_name, cell_ref).
    SheetRef(String, String),
    /// A function name such as `SUM`.
    Function(String),
    /// Opening parenthesis `(`.
    LParen,
    /// Closing parenthesis `)`.
    RParen,
    /// Comma `,` (argument separator).
    Comma,
    /// Colon `:` (range operator).
    Colon,
    /// An arithmetic operator: `+`, `-`, `*`, `/`.
    Operator(char),
    /// A comparison operator: `>`, `<`, `>=`, `<=`, `=`, `<>`.
    Comparison(String),
    /// The `&` string concatenation operator.
    Ampersand,
    /// A string literal (the inner text without quotes).
    StringLiteral(String),
    /// A boolean literal (`TRUE` or `FALSE`).
    Boolean(bool),
}

/// Tokenise a formula string (without leading `=`) into a list of [`Token`]s.
///
/// Handles numeric literals, cell references, function names, parentheses,
/// commas, colons, the four basic arithmetic operators, comparison operators
/// (`>`, `<`, `>=`, `<=`, `=`, `<>`), string concatenation (`&`), string
/// literals in double quotes, and boolean literals (`TRUE` / `FALSE`).
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        match ch {
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '&' => {
                tokens.push(Token::Ampersand);
                i += 1;
            }
            '+' | '*' | '/' => {
                tokens.push(Token::Operator(ch));
                i += 1;
            }
            '-' => {
                // Determine if this is a unary minus (part of a negative number)
                // or a binary subtraction operator.
                let is_unary = tokens.is_empty()
                    || matches!(
                        tokens.last(),
                        Some(
                            Token::LParen
                                | Token::Comma
                                | Token::Operator(_)
                                | Token::Comparison(_)
                        )
                    );
                if is_unary && i + 1 < chars.len() && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.') {
                    // Negative number literal
                    let start = i;
                    i += 1; // skip the '-'
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    if let Ok(n) = num_str.parse::<f64>() {
                        tokens.push(Token::Number(n));
                    }
                } else {
                    tokens.push(Token::Operator('-'));
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Comparison("<>".to_string()));
                    i += 2;
                } else if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Comparison("<=".to_string()));
                    i += 2;
                } else {
                    tokens.push(Token::Comparison("<".to_string()));
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Comparison(">=".to_string()));
                    i += 2;
                } else {
                    tokens.push(Token::Comparison(">".to_string()));
                    i += 1;
                }
            }
            '=' => {
                tokens.push(Token::Comparison("=".to_string()));
                i += 1;
            }
            '"' => {
                // String literal — handle escaped quotes ("") inside strings
                i += 1; // skip opening quote
                let mut s = String::new();
                while i < chars.len() {
                    if chars[i] == '"' {
                        if i + 1 < chars.len() && chars[i + 1] == '"' {
                            // Escaped quote
                            s.push('"');
                            i += 2;
                        } else {
                            break;
                        }
                    } else {
                        s.push(chars[i]);
                        i += 1;
                    }
                }
                tokens.push(Token::StringLiteral(s));
                if i < chars.len() {
                    i += 1; // skip closing quote
                }
            }
            _ if ch.is_ascii_digit() || ch == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                if let Ok(n) = num_str.parse::<f64>() {
                    tokens.push(Token::Number(n));
                }
            }
            _ if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let upper = word.to_ascii_uppercase();

                // Check for boolean literals
                if upper == "TRUE" || upper == "FALSE" {
                    // But only if not followed by '(' (could be a function name)
                    let mut peek = i;
                    while peek < chars.len() && chars[peek].is_whitespace() {
                        peek += 1;
                    }
                    if peek < chars.len() && chars[peek] == '(' {
                        tokens.push(Token::Function(upper));
                    } else {
                        tokens.push(Token::Boolean(upper == "TRUE"));
                    }
                } else {
                    // If the next non-space char is '(', treat it as a function.
                    let mut peek = i;
                    while peek < chars.len() && chars[peek].is_whitespace() {
                        peek += 1;
                    }
                    if peek < chars.len() && chars[peek] == '(' {
                        tokens.push(Token::Function(upper));
                    } else {
                        tokens.push(Token::CellRef(word));
                    }
                }
            }
            '!' => {
                // Sheet reference separator: e.g. "Sheet1" + "!" + "A1"
                // The previous token should be a CellRef containing the sheet name.
                i += 1; // skip '!'
                // Read the cell reference that follows
                let ref_start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '$' || chars[i] == '_')
                {
                    i += 1;
                }
                if i > ref_start {
                    let cell_ref: String = chars[ref_start..i].iter().collect();
                    // Pop the previous CellRef token and convert to SheetRef
                    if let Some(Token::CellRef(sheet_name)) = tokens.last().cloned() {
                        tokens.pop();
                        tokens.push(Token::SheetRef(sheet_name, cell_ref));
                    }
                }
            }
            _ => {
                i += 1; // skip unknown
            }
        }
    }
    tokens
}

/// Convenience alias — currently just calls [`tokenize`].
pub fn parse(input: &str) -> Vec<Token> {
    tokenize(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenize() {
        let tokens = tokenize("SUM(A1:A5)");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0], Token::Function("SUM".to_string()));
        assert_eq!(tokens[1], Token::LParen);
        assert_eq!(tokens[2], Token::CellRef("A1".to_string()));
        assert_eq!(tokens[3], Token::Colon);
        assert_eq!(tokens[4], Token::CellRef("A5".to_string()));
        assert_eq!(tokens[5], Token::RParen);
    }

    #[test]
    fn test_comparison_operators() {
        let tokens = tokenize("A1>=10");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::CellRef("A1".to_string()));
        assert_eq!(tokens[1], Token::Comparison(">=".to_string()));
        assert_eq!(tokens[2], Token::Number(10.0));
    }

    #[test]
    fn test_not_equal() {
        let tokens = tokenize("A1<>B1");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], Token::Comparison("<>".to_string()));
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#"CONCATENATE("hello", " ", "world")"#);
        assert_eq!(tokens[0], Token::Function("CONCATENATE".to_string()));
        assert_eq!(tokens[2], Token::StringLiteral("hello".to_string()));
        assert_eq!(tokens[4], Token::StringLiteral(" ".to_string()));
        assert_eq!(tokens[6], Token::StringLiteral("world".to_string()));
    }

    #[test]
    fn test_boolean_literal() {
        let tokens = tokenize("IF(TRUE, 1, 0)");
        assert_eq!(tokens[0], Token::Function("IF".to_string()));
        assert_eq!(tokens[2], Token::Boolean(true));
    }

    #[test]
    fn test_boolean_as_function() {
        let tokens = tokenize("TRUE()");
        assert_eq!(tokens[0], Token::Function("TRUE".to_string()));
    }

    #[test]
    fn test_negative_number() {
        let tokens = tokenize("SUM(A1, -5)");
        // SUM ( A1 , -5 )
        assert!(tokens.contains(&Token::Number(-5.0)));
    }

    #[test]
    fn test_ampersand() {
        let tokens = tokenize(r#""A" & "B""#);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::StringLiteral("A".to_string()));
        assert_eq!(tokens[1], Token::Ampersand);
        assert_eq!(tokens[2], Token::StringLiteral("B".to_string()));
    }

    #[test]
    fn test_nested_functions() {
        let tokens = tokenize("SUM(IF(A1>0,A1,0),B1)");
        assert_eq!(tokens[0], Token::Function("SUM".to_string()));
        assert_eq!(tokens[2], Token::Function("IF".to_string()));
    }

    #[test]
    fn test_cross_sheet_reference() {
        let tokens = tokenize("Sheet2!A1");
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0],
            Token::SheetRef("Sheet2".to_string(), "A1".to_string())
        );
    }

    #[test]
    fn test_cross_sheet_reference_in_expression() {
        let tokens = tokenize("Sheet2!A1+10");
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0],
            Token::SheetRef("Sheet2".to_string(), "A1".to_string())
        );
        assert_eq!(tokens[1], Token::Operator('+'));
        assert_eq!(tokens[2], Token::Number(10.0));
    }

    #[test]
    fn test_cross_sheet_range_in_sum() {
        let tokens = tokenize("SUM(Sheet2!A1:A5)");
        assert_eq!(tokens[0], Token::Function("SUM".to_string()));
        assert_eq!(tokens[1], Token::LParen);
        assert_eq!(
            tokens[2],
            Token::SheetRef("Sheet2".to_string(), "A1".to_string())
        );
        assert_eq!(tokens[3], Token::Colon);
        assert_eq!(tokens[4], Token::CellRef("A5".to_string()));
        assert_eq!(tokens[5], Token::RParen);
    }
}
