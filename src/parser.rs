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
    fn new(node_value: NodeType, node_next: Option<Box<Node>>) -> Self {
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
                    TokenType::Mul => NodeType::Mul,
                    TokenType::Div => NodeType::Div,
                    _ => panic!("Unexpected token: {:?}", op),
                },
                // 左辺と右辺を正しくリンク
                Some(Box::new(Node::new(
                    node.node_value.clone(),
                    Some(Box::new(*rhs)),
                ))),
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
                    TokenType::Add => NodeType::Add,
                    TokenType::Sub => NodeType::Sub,
                    _ => panic!("Unexpected token: {:?}", op),
                },
                // 左辺と右辺を正しくリンク
                Some(Box::new(Node::new(
                    node.node_value.clone(),
                    Some(Box::new(*rhs)),
                ))),
            ));
        }
        Ok(node)
    }
 
/*
fn parse_function(&mut self) -> R<Box<Node>> {
    // 関数名を取得
    let func_name = self.current_token().token_value().clone();
    self.next_token(); // 関数名をスキップ

    // 引数リストを解析
    if self.current_token().token_type() != TokenType::LeftParen {
        return Err(anyhow::anyhow!("Expected '(' after function name"));
    }
    self.next_token(); // '(' をスキップ

    let mut params = Vec::new();
    while self.current_token().token_type() != TokenType::RightParen {
        if let TokenType::Ident = self.current_token().token_type() {
            let param_name = self.current_token().token_value().clone();
            params.push(Node::new(NodeType::Variable(param_name), None));
            self.next_token();
        }
        if self.current_token().token_type() == TokenType::Comma {
            self.next_token(); // ',' をスキップ
        }
    }
    self.next_token(); // ')' をスキップ

    // 関数本体を解析
    if self.current_token().token_type() != TokenType::LeftCurlyBrace {
        return Err(anyhow::anyhow!(
            "Expected left after function parameters"
        ));
    }
    self.next_token(); // '{' をスキップ

    let body = self.parse_block()?; // ブロックの解析

    Ok(Box::new(Node::new(
        NodeType::Function(func_name, params, Box::new(*body)),
        None,
    )))
}
*/  

fn parse_function(&mut self) -> R<Box<Node>> {
    // 関数名を取得
    let func_name = self.current_token().token_value().clone();
    self.next_token(); // 関数名をスキップ

    // 引数リストを解析
    if self.current_token().token_type() != TokenType::LeftParen {
        return Err(anyhow::anyhow!("Expected '(' after function name"));
    }
    self.next_token(); // '(' をスキップ

    let mut params = Vec::new();
    while self.current_token().token_type() != TokenType::RightParen {
        if let TokenType::Ident = self.current_token().token_type() {
            let param_name = self.current_token().token_value().clone();
            params.push(param_name); // ここを変更
            self.next_token();
        }
        if self.current_token().token_type() == TokenType::Comma {
            self.next_token(); // ',' をスキップ
        }
    }
    self.next_token(); // ')' をスキップ

    // 関数本体を解析
    if self.current_token().token_type() != TokenType::LeftCurlyBrace {
        return Err(anyhow::anyhow!(
            "Expected left after function parameters"
        ));
    }
    self.next_token(); // '{' をスキップ

    let body = self.parse_block()?; // ブロックの解析

    Ok(Box::new(Node::new(
        NodeType::Function(func_name, params, Box::new(*body)), // ここを変更
        None,
    )))
}
    /*
fn factor(&mut self) -> R<Box<Node>> {
    let token = self.current_token().clone();
    match token.token_type() {
        TokenType::Ident => {
            if let Ok(number) = token.token_value().parse::<i32>() {
                self.next_token();
                Ok(Box::new(Node::new(NodeType::Number(number), None)))
            } else {
                self.next_token();
                if self.current_token().token_type() == TokenType::LeftParen {
                    // 関数呼び出し
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
                    
                    if self.current_token().token_type() == TokenType::LeftCurlyBrace {
                        let body = self.parse_block()?; // ブロックの解析
                        return Ok(Box::new(Node::new(
                            NodeType::Function(token.token_value().clone(), args, Box::new(*body)),
                            None,
                        )));
                    }
                    Ok(Box::new(Node::new(
                        NodeType::Call(token.token_value().clone(), args),
                        None,
                    )))
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
    */

fn factor(&mut self) -> R<Box<Node>> {
    let token = self.current_token().clone();
    match token.token_type() {
        TokenType::Ident => {
            if let Ok(number) = token.token_value().parse::<i32>() {
                self.next_token();
                Ok(Box::new(Node::new(NodeType::Number(number), None)))
            } else {
                self.next_token();
                if self.current_token().token_type() == TokenType::LeftParen {
                    // 関数呼び出し
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
                    
                    if self.current_token().token_type() == TokenType::LeftCurlyBrace {
                        let body = self.parse_block()?; // ブロックの解析
                        return Ok(Box::new(Node::new(
                            NodeType::Function(token.token_value().clone(), args.iter().map(|arg| match arg.node_value() {
                                NodeType::Variable(ref name) => name.clone(),
                                _ => "".to_string(),
                            }).collect(), Box::new(*body)),
                            None,
                        )));
                    }
                    Ok(Box::new(Node::new(
                        NodeType::Call(token.token_value().clone(), args),
                        None,
                    )))
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
            if self.current_token().token_type() == TokenType::LeftCurlyBrace{
                self.next_token(); // '{' をスキップ
            }
            let mut nodes = Vec::new();
            while self.current_token().token_type() != TokenType::RightCurlyBrace {
                if self.current_token().token_type() == TokenType::Ident
                    && self.peek_next_token(1).token_type() == TokenType::Equals
                {
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
                && self.peek_next_token(1).token_type() == TokenType::Equals
            {
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
            }else if self.current_token().token_type() == TokenType::Ident
                    && self.current_token().token_value() == "return"{
                    self.next_token();
                    let ret_value = self.expr()?;
                    nodes.push(Node::new(
                        NodeType::Return(ret_value),
                        None,
                    ));
                    break;
            }
            else if self.current_token().token_type() == TokenType::LeftCurlyBrace {
                nodes.push(*self.parse_block()?);
            } else {
                let expr_node = self.expr()?;
                nodes.push(*expr_node);
            }

            if self.current_token().token_type() == TokenType::Semi {
                self.next_token(); // skip ;
            }else {
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
