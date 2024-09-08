use crate::custom_compile_error;
use crate::error::CompilerError;
use crate::types::TokenType;
use anyhow::{anyhow, Context, Result as R};
use log::{error, info, warn};
use property_rs::Property;

#[derive(Debug, Property, Clone)]
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
pub struct Tokenizer {
    #[property(get, set)]
    input: String,
    #[property(get, set)]
    input_vec: Vec<String>,
    #[property(get)]
    line: usize,
    #[property(get)]
    column: usize,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            input: String::new(),
            input_vec: Vec::new(),
            line: 1,
            column: 1,
        }
    }
    pub fn new_with_value_vec(input_vec: Vec<String>) -> Self {
        Tokenizer {
            input: String::new(),
            input_vec,
            line: 0,
            column: 0,
        }
    }

    pub fn new_with_value(input: String) -> Self {
        Tokenizer {
            input,
            input_vec: Vec::new(),
            line: 0,
            column: 0,
        }
    }

    fn is_symbol(&self, c: char) -> bool {
        matches!(
            c,
            '+' | '-' | '*' | '/' | '(' | ')' | ',' | '=' | ';' | '@' | '{' | '}' | '<' | '>'
        )
    }
    fn tokenize_string(&mut self, input: &String) -> R<Vec<Token>, String> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut chars = input.chars().peekable();

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

            let start_line = self.line;
            let start_column = self.column;

            if c.is_digit(10) {
                let mut number = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) {
                        number.push(c);
                        chars.next();
                        self.column += 1;
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
                while let Some(c) = chars.next() {
                    if c == '\'' {
                        self.column += 1;
                        break;
                    }
                    string.push(c);
                    self.column += 1;
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
                while let Some(c) = chars.next() {
                    if c == '\"' {
                        self.column += 1;
                        break;
                    }
                    string.push(c);
                    self.column += 1;
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
                    if next_char == '/' {
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
                                start_line,
                                start_column,
                                &input.clone(),
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
                    }
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
                    }
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
            } else if self.is_symbol(c) {
                let token_type = match c {
                    '+' => TokenType::Add,
                    '-' => TokenType::Sub,
                    '*' => TokenType::Mul,
                    '/' => TokenType::Div,
                    '(' => TokenType::LeftParen,
                    ')' => TokenType::RightParen,
                    '{' => TokenType::LeftCurlyBrace,
                    '}' => TokenType::RightCurlyBrace,
                    '[' => TokenType::LeftSquareBrace,
                    ']' => TokenType::RightSquareBrace,
                    ',' => TokenType::Comma,
                    '=' => TokenType::Equals,
                    '@' => TokenType::AtSign,
                    ';' => TokenType::Semi,
                    _ => {
                        return Err(custom_compile_error!(
                            start_line,
                            start_column,
                            &input.clone(),
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
                    start_line,
                    start_column,
                    &input.clone(),
                    "Unable to process input"
                ));
            }
        }

        Ok(tokens)
    }

    pub fn tokenize(&mut self) -> R<Vec<Token>, String> {
        let mut all_tokens: Vec<Token> = Vec::new();

        // If input_vec is empty, tokenize the input string
        if self.input_vec.is_empty() {
            all_tokens.extend(self.tokenize_string(&self.input.clone())?);
        } else {
            // Tokenize each string in input_vec
            for input in &self.input_vec.clone() {
                all_tokens.extend(self.tokenize_string(input)?);
            }
        }

        all_tokens.push(Token::new(
            String::from(""),
            TokenType::Eof,
            self.line(),
            self.column(),
        ));
        Ok(all_tokens)
    }
}
