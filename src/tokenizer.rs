use crate::types::TokenType;
use anyhow::Result as R;
use log::{error, info, warn};
use property_rs::Property;
#[derive(Debug, Property, Clone)]
pub struct Token {
    #[property(get)]
    token_value: String,
    #[property(get)]
    token_type: TokenType,
}

impl Token {
    fn new(token_value: String, token_type: TokenType) -> Self {
        Token {
            token_value,
            token_type,
        }
    }
}

#[derive(Debug, Property, Clone)]
pub struct Tokenizer {
    #[property(get, set)]
    input: String,

    #[property(get, set)]
    input_vec: Vec<String>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            input: String::new(),
            input_vec: Vec::new(),
        }
    }

    pub fn new_with_value_vec(input_vec: Vec<String>) -> Self {
        Tokenizer {
            input: String::new(),
            input_vec,
        }
    }

    pub fn new_with_value(input: String) -> Self {
        Tokenizer {
            input,
            input_vec: Vec::new(),
        }
    }

    fn is_symbol(&self, c: char) -> bool {
        matches!(
            c,
            '+' | '-' | '*' | '/' | '(' | ')' | ',' | '=' | ';' | '@' | '{' | '}'
        )
    }


    fn tokenize_string(&self, input: &String) -> R<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                continue;
            }

            if c.is_digit(10) {
                let mut number = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) {
                        number.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(number, TokenType::Ident));
            } else if c.is_alphanumeric() || c == '_' {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(ident, TokenType::Ident));
            } else if c == '\'' {
                let mut string = String::new();
                chars.next(); // 開始のクォートをスキップ
                while let Some(c) = chars.next() {
                    if c == '\'' {
                        break;
                    }
                    string.push(c);
                }
                tokens.push(Token::new(string, TokenType::SingleQuote));
            } else if c == '\"' {
                let mut string = String::new();
                chars.next(); // 開始のクォートをスキップ
                while let Some(c) = chars.next() {
                    if c == '\"' {
                        break;
                    }
                    string.push(c);
                }
                tokens.push(Token::new(string, TokenType::DoubleQuote));
            } else if c == '/' {
                chars.next();
                if let Some(&next_char) = chars.peek() {
                    if next_char == '/' {
                        chars.next();
                        let mut comment = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            comment.push(c);
                            chars.next();
                        }
                        tokens.push(Token::new(
                            comment.clone(),
                            TokenType::SingleComment(comment),
                        ));
                    } else if next_char == '*' {
                        chars.next(); // '*' をスキップ
                        let mut comment = String::new();
                        let mut lines = Vec::new();
                        while let Some(c) = chars.next() {
                            if c == '*' {
                                if let Some(&next_char) = chars.peek() {
                                    if next_char == '/' {
                                        chars.next(); // '/' をスキップ
                                        break;
                                    }
                                }
                            }
                            if c == '\n' {
                                lines.push(comment.clone());
                                comment.clear();
                            } else {
                                comment.push(c);
                            }
                        }
                        if !comment.is_empty() {
                            lines.push(comment);
                        }
                        tokens.push(Token::new(lines.join("\n"), TokenType::MultiComment(lines)));
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
                    _ => panic!("Unexpected symbol: {:?}", c),
                };
                tokens.push(Token::new(c.to_string(), token_type));
                chars.next();
            } else {
                panic!("Failed to tokenize: {:?}", c);
            }
        }

        Ok(tokens)
    }

    pub fn tokenize(&mut self) -> R<Vec<Token>> {
        let mut all_tokens: Vec<Token> = Vec::new();

        // If input_vec is empty, tokenize the input string
        if self.input_vec.is_empty() {
            all_tokens.extend(self.tokenize_string(&self.input)?);
        } else {
            // Tokenize each string in input_vec
            for input in &self.input_vec {
                all_tokens.extend(self.tokenize_string(input)?);
            }
        }

        all_tokens.push(Token::new(String::from(""), TokenType::Eof));
        Ok(all_tokens)
    }
}
