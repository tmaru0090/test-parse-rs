use crate::custom_compile_error;
use crate::error::CompilerError;
use crate::types::TokenType;
use anyhow::{anyhow, Context, Result as R};
use log::{error, info, warn};
use property_rs::Property;
use serde::Serialize;
#[derive(Debug, Property, Clone, Serialize)]
pub struct Token {
    #[property(get)]
    token_value: String,
    #[property(get)]
    token_type: TokenType,
    #[property(get)]
    line: usize,
    #[property(get)]
    column: usize,
}

impl Token {
    fn new(token_value: String, token_type: TokenType, line: usize, column: usize) -> Self {
        Token {
            token_value,
            token_type,
            line,
            column,
        }
    }
}

#[derive(Debug, Property, Clone)]
pub struct Lexer {
    #[property(get, set)]
    input_content: String,
    #[property(get, set)]
    input_path: String,
    #[property(get, set)]
    input_content_vec: Vec<String>,
    #[property(get)]
    line: usize,
    #[property(get)]
    column: usize,
}

impl Lexer {
    pub fn new() -> Self {
        Lexer {
            input_path: String::new(),
            input_content: String::new(),
            input_content_vec: Vec::new(),
            line: 1,
            column: 1,
        }
    }
    pub fn new_with_value_vec(input_content_vec: Vec<String>) -> Self {
        Lexer {
            input_path: String::new(),
            input_content: String::new(),
            input_content_vec,
            line: 1,
            column: 1,
        }
    }

    pub fn new_with_value(input_path: String, input_content: String) -> Self {
        Lexer {
            input_path,
            input_content,
            input_content_vec: Vec::new(),
            line: 1,
            column: 1,
        }
    }

    fn is_symbol(&self, c: char) -> bool {
        matches!(
            c,
            '(' | ')' | ',' | '=' | ';' | '@' | '{' | '}' | '<' | '>' | ':' | '[' | ']'
        )
    }
    fn tokenize_string(&mut self, input_content: &String) -> R<Vec<Token>, String> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut chars = input_content.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                if c == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                chars.next();
                continue;
            }

            let start_line = self.line();
            let start_column = self.column();

            if c.is_digit(10) {
                let mut number = String::new();
                let mut has_decimal_point = false;
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) {
                        number.push(c);
                        chars.next();
                        self.column += 1;
                    } else if c == '.' && !has_decimal_point {
                        number.push(c);
                        chars.next();
                        self.column += 1;
                        has_decimal_point = true;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(
                    number,
                    TokenType::Ident,
                    start_line,
                    start_column,
                ));
            } else if c.is_alphanumeric() || c == '_' {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                        self.column += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(
                    ident,
                    TokenType::Ident,
                    start_line,
                    start_column,
                ));
            } else if c == '\'' {
                let mut string = String::new();
                chars.next(); // 開始のクォートをスキップ
                self.column += 1;
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '\'' {
                        self.column += 1;
                        closed = true;
                        break;
                    }
                    string.push(c);
                    self.column += 1;
                }
                if !closed {
                    return Err(custom_compile_error!(
                        "error",
                        start_line,
                        start_column,
                        &self.input_path.clone(),
                        &self.input_content.clone(),
                        "Single quote not closed",
                    ));
                }
                tokens.push(Token::new(
                    string,
                    TokenType::SingleQuote,
                    start_line,
                    start_column,
                ));
            } else if c == '\"' {
                let mut string = String::new();
                chars.next(); // 開始のクォートをスキップ
                self.column += 1;
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '\"' {
                        self.column += 1;
                        closed = true;
                        break;
                    }
                    string.push(c);
                    self.column += 1;
                }
                if !closed {
                    return Err(custom_compile_error!(
                        "error",
                        start_line,
                        start_column,
                        &self.input_path.clone(),
                        &self.input_content.clone(),
                        "Double quote not closed",
                    ));
                }
                tokens.push(Token::new(
                    string,
                    TokenType::DoubleQuote,
                    start_line,
                    start_column,
                ));
            } else if c == '/' {
                chars.next();
                self.column += 1;

                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            "/=".to_string(),
                            TokenType::DivAssign,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else if next_char == '/' {
                        chars.next();
                        self.column += 1;
                        let mut comment = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            comment.push(c);
                            chars.next();
                            self.column += 1;
                        }
                        tokens.push(Token::new(
                            comment.clone(),
                            TokenType::SingleComment(comment, (start_line, start_column)),
                            start_line,
                            start_column,
                        ));
                    } else if next_char == '*' {
                        chars.next(); // '*' をスキップ
                        self.column += 1;
                        let mut comment = String::new();
                        let mut lines = Vec::new();
                        let mut closed = false;
                        while let Some(c) = chars.next() {
                            if c == '*' {
                                if let Some(&next_char) = chars.peek() {
                                    if next_char == '/' {
                                        chars.next(); // '/' をスキップ
                                        self.column += 1;
                                        closed = true;
                                        break;
                                    }
                                }
                            }
                            if c == '\n' {
                                self.line += 1;
                                self.column = 1;
                                lines.push(comment.clone());
                                comment.clear();
                            } else {
                                comment.push(c);
                                self.column += 1;
                            }
                        }
                        if !closed {
                            return Err(custom_compile_error!(
                                "error",
                                start_line,
                                start_column,
                                &self.input_path.clone(),
                                &self.input_content.clone(),
                                "Multi-line comment not closed",
                            ));
                        }
                        if !comment.is_empty() {
                            lines.push(comment);
                        }
                        tokens.push(Token::new(
                            lines.join("\n"),
                            TokenType::MultiComment(lines, (start_line, start_column)),
                            start_line,
                            start_column,
                        ));
                    } else {
                        tokens.push(Token::new(
                            "/".to_string(),
                            TokenType::Div,
                            start_line,
                            start_column,
                        ));
                    }
                } else {
                    tokens.push(Token::new(
                        "/".to_string(),
                        TokenType::Div,
                        start_line,
                        start_column,
                    ));
                }
            } else if c == '=' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            "==".to_string(),
                            TokenType::Eq,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "=".to_string(),
                            TokenType::Equals,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if c == '!' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            "!=".to_string(),
                            TokenType::Ne,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    }
                }
            } else if c == '<' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            "<=".to_string(),
                            TokenType::Le,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "<".to_string(),
                            TokenType::Lt,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if c == '>' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            ">=".to_string(),
                            TokenType::Ge,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            ">".to_string(),
                            TokenType::Gt,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if c == '&' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '&' {
                        tokens.push(Token::new(
                            "&&".to_string(),
                            TokenType::And,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "&".to_string(),
                            TokenType::Reference,
                            start_line,
                            start_column,
                        ));
                    }
                } else {
                    tokens.push(Token::new(
                        "&".to_string(),
                        TokenType::Reference,
                        start_line,
                        start_column,
                    ));
                }
            } else if c == '|' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '|' {
                        tokens.push(Token::new(
                            "||".to_string(),
                            TokenType::Or,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    }
                }
            } else if c == '+' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '+' {
                        tokens.push(Token::new(
                            "++".to_string(),
                            TokenType::Increment,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else if next_char == '=' {
                        tokens.push(Token::new(
                            "+=".to_string(),
                            TokenType::AddAssign,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "+".to_string(),
                            TokenType::Add,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if c == '-' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '-' {
                        tokens.push(Token::new(
                            "--".to_string(),
                            TokenType::Decrement,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else if next_char == '=' {
                        tokens.push(Token::new(
                            "-=".to_string(),
                            TokenType::SubAssign,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else if next_char == '>' {
                        tokens.push(Token::new(
                            "->".to_string(),
                            TokenType::RightArrow,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "-".to_string(),
                            TokenType::Sub,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if c == '*' {
                chars.next();
                self.column += 1;
                if let Some(&next_char) = chars.peek() {
                    if next_char == '=' {
                        tokens.push(Token::new(
                            "*=".to_string(),
                            TokenType::MulAssign,
                            start_line,
                            start_column,
                        ));
                        self.column += 1;
                        chars.next();
                    } else {
                        tokens.push(Token::new(
                            "*".to_string(),
                            TokenType::Mul,
                            start_line,
                            start_column,
                        ));
                    }
                }
            } else if self.is_symbol(c) {
                let token_type = match c {
                    '(' => TokenType::LeftParen,
                    ')' => TokenType::RightParen,
                    '{' => TokenType::LeftCurlyBrace,
                    '}' => TokenType::RightCurlyBrace,
                    '[' => TokenType::LeftSquareBrace,
                    ']' => TokenType::RightSquareBrace,
                    ',' => TokenType::Conma,
                    '=' => TokenType::Equals,
                    '@' => TokenType::AtSign,
                    ';' => TokenType::Semi,
                    ':' => TokenType::Colon,
                    _ => {
                        return Err(custom_compile_error!(
                            "error",
                            start_line,
                            start_column,
                            &self.input_path.clone(),
                            &self.input_content.clone(),
                            "Unexpected symbol"
                        ));
                    }
                };
                tokens.push(Token::new(
                    c.to_string(),
                    token_type,
                    start_line,
                    start_column,
                ));
                chars.next();
                self.column += 1;
            } else {
                return Err(custom_compile_error!(
                    "error",
                    start_line,
                    start_column,
                    &self.input_path.clone(),
                    &self.input_content.clone(),
                    "Unable to process input_content"
                ));
            }
        }

        Ok(tokens)
    }
    pub fn tokenize(&mut self) -> R<Vec<Token>, String> {
        let mut all_tokens: Vec<Token> = Vec::new();

        // If input_content_vec is empty, tokenize the input_content string
        if self.input_content_vec.is_empty() {
            all_tokens.extend(self.tokenize_string(&self.input_content.clone())?);
        } else {
            // Tokenize each string in input_content_vec
            for input_content in &self.input_content_vec.clone() {
                all_tokens.extend(self.tokenize_string(input_content)?);
            }
        }

        // Get the position of the last token
        let (last_line, last_column) = if let Some(last_token) = all_tokens.last() {
            (last_token.line, last_token.column)
        } else {
            (self.line(), self.column())
        };

        // Push EOF token with the position of the last token
        all_tokens.push(Token::new(
            String::from(""),
            TokenType::Eof,
            last_line,
            last_column,
        ));
        Ok(all_tokens)
    }
}
