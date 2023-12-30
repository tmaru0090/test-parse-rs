use std::collections::HashMap;
#[derive(PartialEq, Eq, Debug, Clone)]
enum TokenType {
    Int,     // 整数値
    IntDecl, // int宣言
    Ident,   // 識別子
    Add,     // +
    Sub,     // -
    Mul,     // *
    Div,     // /
    LParen,  // (
    RParen,  // )
    Assign,  // =
    Semi,    // ;
    Eof,     // トークンの終わり
    Error,   // エラー時
}
#[derive(Debug, Clone)]
struct Token {
    token_type: TokenType,
    value: String,
}
impl Token {
    fn new(token_type: TokenType, value: String) -> Token {
        Token { token_type, value }
    }
}

fn tokenize(input: &String) -> Result<Vec<Token>, String> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let mut c = input.chars().nth(pos).expect("Index out of bounds");
        if c == ' ' {
            pos += 1;
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
        } else if c.is_alphabetic() {
            let mut ident = String::new();
            while pos < input.len() && c.is_alphabetic() {
                ident.push(c);
                pos += 1;
                if pos < input.len() {
                    c = input.chars().nth(pos).expect("Index out of bounds");
                }
            }
            if ident == "int" {
                tokens.push(Token::new(TokenType::IntDecl, ident));
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

#[derive(PartialEq, Eq, Debug, Clone)]
enum NodeType {
    Var(String), // 変数
    VarDecl,     // 変数宣言
    VarAssign,   // 変数代入
    Add,         // +
    Sub,         // -
    Mul,         // *
    Div,         // /
    Num(String), // 値
    Error,       // エラー
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct Node {
    node_type: NodeType,
    value: String,
    child: Vec<Box<Node>>,
}
impl Node {
    fn new(node_type: NodeType, child: Vec<Box<Node>>, value: String) -> Node {
        Node {
            node_type,
            value,
            child,
        }
    }
}
struct Parser<'a> {
    tokens: &'a Vec<Token>,
    pos: usize,
    variables: HashMap<String, i64>, // 変数名と値の関連付けを管理するHashMap
}
impl<'a> Parser<'a> {
    fn get_variables(&self) -> HashMap<String, i64> {
        self.variables.clone()
    }
    fn new(tokens: &'a Vec<Token>) -> Parser<'a> {
        Parser {
            tokens,
            pos: 0,
            variables: HashMap::new(),
        }
    }
    fn current_tokens(&self) -> Token {
        self.tokens[self.pos].clone()
    }
    fn next_tokens(&mut self) {
        self.pos += 1;
    }

    fn factor(&mut self) -> Result<Box<Node>, String> {
        let current_token = self.current_tokens().clone();
        match current_token.token_type {
            TokenType::Int => {
                self.next_tokens();
                Ok(Box::new(Node::new(
                    NodeType::Num(current_token.value.clone()),
                    vec![],
                    current_token.value.clone(),
                )))
            }
            TokenType::IntDecl => {
                self.next_tokens();
                let ident = self.current_tokens().value.clone(); // 変数名を取得する
                self.next_tokens();
                let value_node = self.expr()?; // 変数に代入される式を取得する
                                               // 変数の値の評価はここでは行わない
                Ok(Box::new(Node::new(
                    NodeType::VarDecl,
                    vec![
                        Box::new(Node::new(
                            NodeType::Var(ident.clone()),
                            vec![],
                            ident.clone(),
                        )),
                        value_node,
                    ],
                    ident,
                )))
            }

            TokenType::Ident => {
                self.next_tokens();
                let ident = self.current_tokens().value.clone(); // 変数名を取得する
                                                                 // 変数の値の取得はここでは行わない
                Ok(Box::new(Node::new(
                    NodeType::Var(ident.clone()),
                    vec![],
                    ident,
                )))
            }
            TokenType::Assign => {
                self.next_tokens();
                let ident = self.current_tokens().value.clone(); // 変数名を取得する
                self.next_tokens();
                let value_node = self.expr()?; // 代入される式を取得する
                                               // 変数の値の評価はここでは行わない
                Ok(Box::new(Node::new(
                    NodeType::VarAssign,
                    vec![
                        Box::new(Node::new(NodeType::Var(ident.clone()), vec![], ident)),
                        value_node,
                    ],
                    current_token.value,
                )))
            }
            TokenType::LParen => {
                self.next_tokens();
                let node = self.expr()?;
                if self.current_tokens().token_type != TokenType::RParen {
                    panic!(
                        "Expected closing parenthesis ')' but found {:?}",
                        self.current_tokens()
                    );
                }
                self.next_tokens();
                Ok(node)
            }
            TokenType::Eof => {
                // Eofトークンに達した場合は、現在のノードを返す
                Ok(Box::new(Node::new(
                    NodeType::Error,
                    vec![],
                    "Unexpected end of input".to_string(),
                )))
            }
            _ => panic!("Unexpected token: {:?}", current_token),
        }
    }
    // expr()とterm()メソッドの修正
    fn expr(&mut self) -> Result<Box<Node>, String> {
        let mut node = self.term()?;
        while self.current_tokens().token_type == TokenType::Add
            || self.current_tokens().token_type == TokenType::Sub
        {
            let current_token = self.current_tokens().clone();
            self.next_tokens();
            node = Box::new(Node::new(
                match current_token.token_type {
                    TokenType::Add => NodeType::Add,
                    TokenType::Sub => NodeType::Sub,
                    TokenType::Eof => {
                        return Ok(node); // Eofトークンに達したら処理を終了して結果を返す
                    }
                    _ => unreachable!(),
                },
                vec![node, self.term()?],
                current_token.value,
            ));
        }
        Ok(node)
    }

    fn term(&mut self) -> Result<Box<Node>, String> {
        let mut node = self.factor()?;
        while self.current_tokens().token_type == TokenType::Mul
            || self.current_tokens().token_type == TokenType::Div
        {
            let current_token = self.current_tokens().clone();
            self.next_tokens();
            node = Box::new(Node::new(
                match current_token.token_type {
                    TokenType::Mul => NodeType::Mul,
                    TokenType::Div => NodeType::Div,
                    TokenType::Eof => {
                        return Ok(node); // Eofトークンに達したら処理を終了して結果を返す
                    }
                    _ => unreachable!(),
                },
                vec![node, self.factor()?],
                current_token.value,
            ));
        }
        Ok(node)
    }
    // 式の評価
    fn eval(&self, node: &Node) -> Result<i64, String> {
        match &node.node_type {
            NodeType::Add => {
                let left = self.eval(&node.child[0])?;
                let right = self.eval(&node.child[1])?;
                Ok(left + right)
            }
            NodeType::Sub => {
                let left = self.eval(&node.child[0])?;
                let right = self.eval(&node.child[1])?;
                Ok(left - right)
            }
            NodeType::Mul => {
                let left = self.eval(&node.child[0])?;
                let right = self.eval(&node.child[1])?;
                Ok(left * right)
            }
            NodeType::Div => {
                let left = self.eval(&node.child[0])?;
                let right = self.eval(&node.child[1])?;
                if right == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(left / right)
                }
            }
            NodeType::Var(variable_name) => {
                if let Some(value) = self.variables.get(variable_name) {
                    Ok(*value) // 変数名に対応する値を返す
                } else {
                    Err("Variable not found".to_string())
                }
            }
            NodeType::Num(expression) => {
                // 式を評価する処理
                // ここでは単純に式をi64にパースして返す
                expression
                    .parse::<i64>()
                    .map_err(|_| "Invalid expression".to_string())
            }
            _ => Err("Invalid operation".to_string()),
        }
    }
}
struct Decoder {}
impl Decoder {
    fn new() -> Decoder {
        Decoder {}
    }
}

fn main() -> Result<(), String> {
    let src = String::from("12+1");
    let tokens = tokenize(&src)?;
    let mut parser = Parser::new(&tokens);
    println!("tokens: {:?}", tokens);
    let node = parser.expr()?;
    println!("node: {:?}", node);
    // 式を評価して結果を表示
    let result = parser.eval(&node)?;
    println!("Result: {}", result);
    println!("");
    Ok(())
}
