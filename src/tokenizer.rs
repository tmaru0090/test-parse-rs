use crate::types::*;
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
}
impl Token {
    fn new(token_type: TokenType, value: String) -> Token {
        Token { token_type, value }
    }
}
pub struct Tokenizer {}
impl Tokenizer {
    pub fn new() -> Tokenizer {
        Tokenizer {}
    }
    pub fn tokenize(&self, input: &String) -> Result<Vec<Token>, String> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut pos = 0;
        while pos < input.len() {
            let mut c = input.chars().nth(pos).expect("Index out of bounds");
            if c == ' ' || c == '\n' || c == '\t' {
                pos += 1;
                continue;
            } else if c.is_digit(10) {
                let mut num = String::new();
                while pos < input.len() && c.is_digit(10) {
                    num.push(c);
                    pos += 1;
                    if pos < input.len() {
                        c = input.chars().nth(pos).expect("Index out of bounds");
                    }
                }
                tokens.push(Token::new(TokenType::Int, num));
            } else if c.is_alphabetic() || c.is_alphanumeric() || c == '_' {
                let mut ident = String::new();
                while pos < input.len() && (c.is_alphabetic() || c.is_alphanumeric() || c == '_') {
                    ident.push(c);
                    pos += 1;
                    if pos < input.len() {
                        c = input.chars().nth(pos).expect("Index out of bounds");
                    }
                }
                //let first_string = ident.chars().next().unwrap();
                if ident == "let" || ident == "l" {
                    tokens.push(Token::new(TokenType::LetDecl, ident));
                } else {
                    tokens.push(Token::new(TokenType::Ident, ident));
                }
            } else {
                match c {
                    ';' => tokens.push(Token::new(TokenType::Semi, ";".to_string())),
                    '=' => tokens.push(Token::new(TokenType::Assign, "=".to_string())),
                    '+' => tokens.push(Token::new(TokenType::Add, "+".to_string())),
                    '-' => tokens.push(Token::new(TokenType::Sub, "-".to_string())),
                    '*' => tokens.push(Token::new(TokenType::Mul, "*".to_string())),
                    '/' => tokens.push(Token::new(TokenType::Div, "/".to_string())),
                    '(' => tokens.push(Token::new(TokenType::LParen, "(".to_string())),
                    ')' => tokens.push(Token::new(TokenType::RParen, ")".to_string())),
                    '{' => tokens.push(Token::new(TokenType::LBlockDelimiter, "{".to_string())),
                    '}' => tokens.push(Token::new(TokenType::RBlockDelimiter, "}".to_string())),
                    _ => {
                        tokens.push(Token::new(TokenType::Error, "Error!".to_string()));
                        return Err(format!("この文字はトークンではありませんよ {}", c));
                    }
                }
                pos += 1;
            }
        }
        tokens.push(Token::new(TokenType::Eof, "".to_string()));
        Ok(tokens)
    }
}
