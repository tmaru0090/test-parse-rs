use crate::tokenizer::Token;
use crate::types::{NodeType, TokenType};
use anyhow::Result as R;
use log::{error, info, warn};
use property_rs::Property;

#[derive(Debug, PartialEq, Clone, Property)]
pub struct Node {
    #[property(get)]
    node_value: NodeType,
    #[property(get)]
    node_next: Option<Box<Node>>,
}

impl Node {
    pub fn new(node_value: NodeType, node_next: Option<Box<Node>>) -> Self {
        Node {
            node_value,
            node_next,
        }
    }
}

pub struct Parser<'a> {
    tokens: &'a Vec<Token>,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>) -> Self {
        Parser { tokens, i: 0 }
    }

    pub fn new_add(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Add(left, right), None);

        Box::new(node)
    }
    pub fn new_sub(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Sub(left, right), None);
        Box::new(node)
    }
    pub fn new_mul(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Mul(left, right), None);

        Box::new(node)
    }
    pub fn new_div(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Div(left, right), None);

        Box::new(node)
    }

    pub fn new_variable(name: String, expr: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Variable(name.clone()), Some(expr));
        Box::new(node)
    }

    pub fn new_return(expr: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Return(expr), None);
        Box::new(node)
    }

    pub fn new_block(block: Vec<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Block(block), None);
        Box::new(node)
    }

    pub fn new_assign(left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(NodeType::Assign(left, right), None);
        Box::new(node)
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.i]
    }

    fn peek_next_token(&mut self, i: usize) -> Token {
        if (self.i + i) < self.tokens.len() {
            let token = self.tokens[self.i + i].clone();
            token
        } else {
            panic!("Index out of bounds");
        }
    }
    fn previous_token(&mut self, i: usize) -> Token {
        if self.i >= i {
            let token = self.tokens[self.i - i].clone();
            token
        } else {
            panic!("Index out of bounds");
        }
    }

    fn next_token(&mut self) {
        self.i += 1;
    }
    fn term(&mut self) -> R<Box<Node>> {
        let mut node = self.factor()?;
        while matches!(
            self.current_token().token_type(),
            TokenType::Mul | TokenType::Div
        ) {
            let op = self.current_token().clone();
            self.next_token();
            let rhs = self.factor()?;
            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Mul => NodeType::Mul(node, rhs),
                    TokenType::Div => NodeType::Div(node, rhs),
                    _ => panic!("Unexpected token: {:?}", op),
                },
                None,
            ));
        }
        Ok(node)
    }

    fn expr(&mut self) -> R<Box<Node>> {
        let mut node = self.term()?;
        while matches!(
            self.current_token().token_type(),
            TokenType::Add | TokenType::Sub
        ) {
            let op = self.current_token().clone();
            self.next_token();
            let rhs = self.term()?;
            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Add => NodeType::Add(node, rhs),
                    TokenType::Sub => NodeType::Sub(node, rhs),
                    _ => panic!("Unexpected token: {:?}", op),
                },
                None,
            ));
        }
        Ok(node)
    }
    fn parse_function_call(&mut self, token: Token) -> R<Box<Node>> {
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        while self.current_token().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().token_type() == TokenType::Comma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ
        Ok(Box::new(Node::new(
            NodeType::Call(token.token_value().clone(), args),
            None,
        )))
    }

    fn parse_function_definition(&mut self) -> R<Box<Node>> {
        self.next_token(); // 'fn' をスキップ
        let name = self.current_token().token_value().clone();
        self.next_token(); // 関数名をスキップ
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        while self.current_token().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().token_type() == TokenType::Comma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeType::Function(
                name,
                args.iter()
                    .map(|arg| match arg.node_value() {
                        NodeType::Variable(ref name) => name.clone(),
                        _ => "".to_string(),
                    })
                    .collect(),
                Box::new(*body),
            ),
            None,
        )))
    }

    fn factor(&mut self) -> R<Box<Node>> {
        let token = self.current_token().clone();
        match token.token_type() {
            TokenType::DoubleQuote | TokenType::SingleQuote => {
                if let Ok(string) = token.token_value().parse::<String>() {
                    self.next_token();
                    Ok(Box::new(Node::new(NodeType::String(string), None)))
                } else {
                    return Err(anyhow::anyhow!(
                        "Unexpected end of input, no closing DoubleQuote or SingleQuote found"
                    ));
                }
            }
            TokenType::Ident => {
                if token.token_value() == "fn" {
                    self.parse_function_definition()
                } else if let Ok(number) = token.token_value().parse::<i32>() {
                    self.next_token();
                    Ok(Box::new(Node::new(NodeType::Number(number), None)))
                } else {
                    self.next_token();
                    if self.current_token().token_type() == TokenType::LeftParen {
                        self.parse_function_call(token)
                    } else {
                        Ok(Box::new(Node::new(
                            NodeType::Variable(token.token_value().clone()),
                            None,
                        )))
                    }
                }
            }
            TokenType::LeftParen => {
                self.next_token();
                let expr = self.expr()?;
                if self.current_token().token_type() != TokenType::RightParen {
                    let e = anyhow::anyhow!("no closing parenthesis in factor: {:?}", token);
                    Err(e)
                } else {
                    self.next_token();
                    Ok(expr)
                }
            }
            TokenType::LeftCurlyBrace => self.parse_block(),
            _ => Err(anyhow::anyhow!("Unexpected token in factor: {:?}", token)),
        }
    }
    fn parse_block(&mut self) -> R<Box<Node>> {
        if self.current_token().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // '{' をスキップ
        }
        let mut nodes = Vec::new();
        while self.current_token().token_type() != TokenType::RightCurlyBrace {
            if self.current_token().token_type() == TokenType::Eof {
                return Err(anyhow::anyhow!(
                    "Unexpected end of input, no closing curly brace found"
                ));
            }
            // 変数代入文
            if self.current_token().token_type() == TokenType::Ident
                && self.current_token().token_value() == "let"
                && self.peek_next_token(2).token_type() == TokenType::Equals
            {
                self.next_token(); // let
                                   // 代入式
                let var = self.current_token().token_value().clone();
                self.next_token(); // =
                self.next_token(); // move to value
                let value_node = self.expr()?;
                nodes.push(Node::new(
                    NodeType::Assign(
                        Box::new(Node::new(NodeType::Variable(var), None)),
                        value_node,
                    ),
                    None,
                ));
            } else {
                let expr = self.parse_statement()?;
                nodes.extend(expr);
            }
            if self.current_token().token_type() == TokenType::Semi {
                self.next_token();
            }
        }
        if self.current_token().token_type() != TokenType::RightCurlyBrace {
            let e = anyhow::anyhow!("no closing curly brace in block");
            Err(e)
        } else {
            self.next_token(); // '}' をスキップ
            Ok(Box::new(Node::new(NodeType::Block(nodes), None)))
        }
    }
    fn parse_statement(&mut self) -> R<Vec<Node>> {
        let mut nodes = Vec::new();
        while self.current_token().token_type() != TokenType::Eof {
            // 変数代入文
            if self.current_token().token_type() == TokenType::Ident
                && self.current_token().token_value() == "let"
                && self.peek_next_token(2).token_type() == TokenType::Equals
            {
                self.next_token(); // let
                                   // 代入式
                let var = self.current_token().token_value().clone();
                self.next_token(); // =
                self.next_token(); // move to value
                let value_node = self.expr()?;
                nodes.push(Node::new(
                    NodeType::Assign(
                        Box::new(Node::new(NodeType::Variable(var), None)),
                        value_node,
                    ),
                    None,
                ));
            } else if self.current_token().token_type() == TokenType::Ident
                && self.current_token().token_value() == "return"
            {
                self.next_token();
                let ret_value = self.expr()?;
                nodes.push(Node::new(NodeType::Return(ret_value), None));
                break;
            } else if self.current_token().token_type() == TokenType::LeftCurlyBrace {
                nodes.push(*self.parse_block()?);
            } else {
                let expr_node = self.expr()?;
                nodes.push(*expr_node);
            }

            if self.current_token().token_type() == TokenType::Semi {
                self.next_token(); // skip ;
            } else {
                warn!("Missing semicolon at end of the statement.");
            }
        }
        Ok(nodes)
    }
    pub fn parse(&mut self) -> R<Vec<Node>> {
        let mut nodes: Vec<Node> = Vec::new();
        nodes = self.parse_statement()?;
        Ok(nodes)
    }
}
