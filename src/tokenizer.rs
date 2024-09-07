/*
use crate::types::TokenType;
use anyhow::Result as R;
use property_rs::Property;
use std::collections::HashMap;

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
    #[property(get,set)]
    input: String,
    input_vec:Vec<String>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer{input:String::new(),input_vec:Vec::new()}
    }
    pub fn new_with_value_vec(input_vec:Vec<String>)->Self{
        Tokenizer{input:String::new(),input_vec}
    }
    pub fn new_with_value(input: String) -> Self {
        Tokenizer { input,input_vec:Vec::new() }
    }
    fn is_symbol(&self, c: char) -> bool {
        match c {
            '+' | '-' | '*' | '/' | '(' | ')' | ',' | '=' | ';' | '@' | '{' | '}' => true,
            _ => false,
        }
    }

    pub fn tokenize(&mut self) -> R<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();
        let input = self.input.clone();
        let mut i = 0;

        while i < input.len() {
            let c = input.chars().nth(i).expect("index out of bounds.");

            if c == ' ' || c == '\n' || c == '\t' {
                i += 1;
                continue;
            }

            if c.is_digit(10) {
                let mut number = String::new();
                while i < input.len() {
                    let c = input.chars().nth(i).expect("index out of bounds.");
                    if c.is_digit(10) {
                        number.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(number, TokenType::Ident));
            } else if c.is_alphanumeric() || c == '_' {
                let mut ident = String::new();
                while i < input.len() {
                    let c = input.chars().nth(i).expect("index out of bounds.");
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(ident, TokenType::Ident));
            } else if self.is_symbol(c) {
                let token_type = match c {
                    '+' => TokenType::Add,
                    '-' => TokenType::Sub,
                    '*' => TokenType::Mul,
                    '/' => TokenType::Div,
                    '(' => TokenType::LeftParen,
                    ')' => TokenType::RightParen,
                    ',' => TokenType::Comma,
                    '=' => TokenType::Equals,
                    '@' => TokenType::AtSign,
                    ';' => TokenType::Semi,
                    '{' => TokenType::LeftCurlyBreces,
                    '}' => TokenType::RightCurlyBreces,
                    _ => panic!("Unexpected symbol: {:?}", c),
                };
                tokens.push(Token::new(c.to_string(), token_type));
                i += 1;
            } else {
                panic!("Failed to tokenize: {:?}", c);
            }
        }
        tokens.push(Token::new(String::from(""), TokenType::Eof));
        Ok(tokens)
    }
}*/

use crate::types::TokenType;
use anyhow::Result as R;
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
        let mut i = 0;

        while i < input.len() {
            let c = input.chars().nth(i).expect("index out of bounds.");

            if c == ' ' || c == '\n' || c == '\t' || c == '\r' {
                i += 1;
                continue;
            }

            if c.is_digit(10) {
                let mut number = String::new();
                while i < input.len() {
                    let c = input.chars().nth(i).expect("index out of bounds.");
                    if c.is_digit(10) {
                        number.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(number, TokenType::Ident));
            } else if c.is_alphanumeric() || c == '_' {
                let mut ident = String::new();
                while i < input.len() {
                    let c = input.chars().nth(i).expect("index out of bounds.");
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(ident, TokenType::Ident));
            } else if self.is_symbol(c) {
                let token_type = match c {
                    '+' => TokenType::Add,
                    '-' => TokenType::Sub,
                    '*' => TokenType::Mul,
                    '/' => TokenType::Div,
                    '(' => TokenType::LeftParen,
                    ')' => TokenType::RightParen,
                    ',' => TokenType::Comma,
                    '=' => TokenType::Equals,
                    '@' => TokenType::AtSign,
                    ';' => TokenType::Semi,
                    '{' => TokenType::LeftCurlyBrace,
                    '}' => TokenType::RightCurlyBrace,
                    _ => panic!("Unexpected symbol: {:?}", c),
                };
                tokens.push(Token::new(c.to_string(), token_type));
                i += 1;
            } else {
                panic!("Failed to tokenize: {:?}", c);
            }
        }

        //tokens.push(Token::new(String::from(""), TokenType::Eof));
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
