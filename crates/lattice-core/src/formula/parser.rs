/// Tokens produced by the formula tokeniser.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A numeric literal.
    Number(f64),
    /// A cell reference such as `A1`.
    CellRef(String),
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
    /// A string literal (the inner text without quotes).
    StringLiteral(String),
}

/// Tokenise a formula string (without leading `=`) into a list of [`Token`]s.
///
/// This is intentionally simple — it handles numeric literals, cell references,
/// function names, parentheses, commas, colons, and the four basic arithmetic
/// operators.
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
            '+' | '-' | '*' | '/' => {
                tokens.push(Token::Operator(ch));
                i += 1;
            }
            '"' => {
                // String literal
                i += 1; // skip opening quote
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
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
                // If the next non-space char is '(', treat it as a function.
                let mut peek = i;
                while peek < chars.len() && chars[peek].is_whitespace() {
                    peek += 1;
                }
                if peek < chars.len() && chars[peek] == '(' {
                    tokens.push(Token::Function(word.to_ascii_uppercase()));
                } else {
                    tokens.push(Token::CellRef(word));
                }
            }
            _ => {
                i += 1; // skip unknown
            }
        }
    }
    tokens
}

/// Placeholder for a full recursive-descent parser. Currently the evaluator
/// works directly on tokens.
pub fn parse(input: &str) -> Vec<Token> {
    tokenize(input)
}
