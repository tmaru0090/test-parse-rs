use crate::compile_error;
use crate::error::*;
use crate::lexer::Token;
use crate::types::{NodeValue, TokenType};
use anyhow::{anyhow, Context, Result as R};
use log::{error, info, warn};
use property_rs::Property;
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Clone, Property, Serialize, Deserialize)]
pub struct Node {
    #[property(get)]
    node_value: NodeValue,
    #[property(get)]
    node_next: Option<Box<Node>>,
    #[property(get)]
    line: usize,
    #[property(get)]
    column: usize,
    #[property(get)]
    is_statement: bool,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            node_value: NodeValue::default(),
            node_next: None,
            line: 0,
            column: 0,
            is_statement: false,
        }
    }
}

impl Node {
    pub fn new(
        node_value: NodeValue,
        node_next: Option<Box<Node>>,
        line: usize,
        column: usize,
    ) -> Self {
        Node {
            node_value,
            node_next,
            line,
            column,
            is_statement: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parser<'a> {
    input_content: String,
    input_path: String,
    tokens: &'a Vec<Token>,
    i: usize,
    is_statement: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>, input_path: &str, input_content: String) -> Self {
        Parser {
            tokens,
            i: 0,
            input_path: input_path.to_string(),
            input_content,
            is_statement: false,
        }
    }
    pub fn input_content(&self) -> String {
        self.input_content.clone()
    }
    pub fn input_path(&self) -> String {
        self.input_path.clone()
    }

    pub fn new_add(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Add(left, right),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }
    pub fn new_sub(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Sub(left, right),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }
    pub fn new_mul(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Mul(left, right),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );

        Box::new(node)
    }
    pub fn new_div(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Div(left, right),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );

        Box::new(node)
    }
    pub fn new_int(&self, value: i64) -> Box<Node> {
        let node = Node::new(
            NodeValue::Int(value),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }
    pub fn new_float(&self, value: f64) -> Box<Node> {
        let node = Node::new(
            NodeValue::Float(value),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_variable(&self, name: String, expr: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Variable(name.clone()),
            Some(expr),
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_return(&self, expr: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Return(expr),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_empty(&self) -> Box<Node> {
        let node = Node::new(
            NodeValue::Empty,
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }
    pub fn new_block(&self, block: Vec<Box<Node>>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Block(block),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_assign(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Assign(left, right, Box::new(Node::default())),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn from_parse(
        tokens: &Vec<Token>,
        input_path: &str,
        input_content: String,
    ) -> R<Vec<Box<Node>>, String> {
        let mut parser = Parser::new(tokens, input_path, input_content);
        parser.parse()
    }
    /*
        fn current_token(&self) -> &Token {
            &self.tokens[self.i]
        }
    */

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.i)
    }

    fn is_next_token_eof(&self) -> bool {
        matches!(self.tokens.get(self.i + 1), Some(token) if token.token_type() == TokenType::Eof)
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
    fn term(&mut self) -> R<Box<Node>, String> {
        let mut node = self.factor()?;
        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Mul | TokenType::Div | TokenType::MulAssign | TokenType::DivAssign
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.factor()?;
            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Mul => NodeValue::Mul(node, rhs),
                    TokenType::Div => NodeValue::Div(node, rhs),
                    TokenType::MulAssign => NodeValue::MulAssign(node, rhs),
                    TokenType::DivAssign => NodeValue::DivAssign(node, rhs),
                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                },
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn expr(&mut self) -> R<Box<Node>, String> {
        let mut node = self.term()?;
        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Add
                | TokenType::Sub
                | TokenType::AddAssign
                | TokenType::SubAssign
                | TokenType::Increment
                | TokenType::Decrement
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.term()?;
            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Add => NodeValue::Add(node, rhs),
                    TokenType::Sub => NodeValue::Sub(node, rhs),
                    TokenType::AddAssign => NodeValue::AddAssign(node, rhs),
                    TokenType::SubAssign => NodeValue::SubAssign(node, rhs),
                    TokenType::Increment => NodeValue::Increment(node),
                    TokenType::Decrement => NodeValue::Decrement(node),
                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                },
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn parse_function_call(&mut self, token: Token, is_system: bool) -> R<Box<Node>, String> {
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ

        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
        }
        Ok(Box::new(Node {
            node_value: NodeValue::Call(token.token_value().clone(), args, is_system),
            node_next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }

    fn parse_callback_function_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'callback' をスキップ
        if self.current_token().unwrap().token_value() == "fn" {
            self.next_token(); // 'fn' をスキップ
            let mut is_system = false;
            if self.current_token().unwrap().token_type() == TokenType::AtSign {
                self.next_token(); // '@' をスキップ
                is_system = true;
            }

            let name = self.current_token().unwrap().token_value().clone();
            self.next_token(); // 関数名をスキップ
            self.next_token(); // '(' をスキップ
            let mut args: Vec<(Box<Node>, String)> = Vec::new();
            let mut return_type = self.new_empty();
            while self.current_token().unwrap().token_type() != TokenType::RightParen {
                let arg = self.expr()?;
                let mut data_type = self.new_empty();
                if self.current_token().unwrap().token_type() == TokenType::Colon {
                    self.next_token(); // ':' をスキップ
                    data_type = self.expr()?;
                    data_type = Box::new(Node::new(
                        NodeValue::DataType(data_type),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    ));
                }
                let arg_name = match arg.node_value() {
                    NodeValue::Variable(ref name) => name.clone(),
                    _ => return Err("Invalid argument name".to_string()),
                };
                args.push((data_type, arg_name));
                if self.current_token().unwrap().token_type() == TokenType::Conma {
                    self.next_token(); // ',' をスキップ
                }
            }
            self.next_token(); // ')' をスキップ
            if self.current_token().unwrap().token_type() == TokenType::RightArrow {
                return_type = self.parse_return_type()?;
            }
            let body = self.parse_block()?; // ブロックの解析

            return Ok(Box::new(Node::new(
                NodeValue::CallBackFunction(name, args, Box::new(*body), return_type, is_system),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )));
        }
        Ok(Box::new(Node::default()))
    }
    fn parse_function_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'fn' をスキップ
        let mut is_system = false;
        if self.current_token().unwrap().token_type() == TokenType::AtSign {
            self.next_token(); // '@' をスキップ
            is_system = true;
        }
        let name = self.current_token().unwrap().token_value().clone();
        self.next_token(); // 関数名をスキップ
        self.next_token(); // '(' をスキップ
        let mut args: Vec<(Box<Node>, String)> = Vec::new();
        let mut return_type = self.new_empty();
        while self.current_token().unwrap().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            let mut data_type = self.new_empty();
            if self.current_token().unwrap().token_type() == TokenType::Colon {
                self.next_token(); // ':' をスキップ
                data_type = self.expr()?;
                data_type = Box::new(Node::new(
                    NodeValue::DataType(data_type),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                ));
            }
            let arg_name = match arg.node_value() {
                NodeValue::Variable(ref name) => name.clone(),
                _ => return Err("Invalid argument name".to_string()),
            };
            args.push((data_type, arg_name));
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ
        if self.current_token().unwrap().token_type() == TokenType::RightArrow {
            return_type = self.parse_return_type()?;
        }
        let body = self.parse_block()?; // ブロックの解析

        Ok(Box::new(Node::new(
            NodeValue::Function(name, args, Box::new(*body), return_type, is_system),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_condition(&mut self) -> R<Box<Node>, String> {
        let mut node = self.expr()?; // 基本の式を解析

        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Eq
                | TokenType::Ne
                | TokenType::Lt
                | TokenType::Gt
                | TokenType::Le
                | TokenType::Ge
                | TokenType::And
                | TokenType::Or
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.expr()?; // 条件演算子の右側の式を解析

            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Eq => NodeValue::Eq(node, rhs),
                    TokenType::Ne => NodeValue::Ne(node, rhs),
                    TokenType::Lt => NodeValue::Lt(node, rhs),
                    TokenType::Gt => NodeValue::Gt(node, rhs),
                    TokenType::Le => NodeValue::Le(node, rhs),
                    TokenType::Ge => NodeValue::Ge(node, rhs),
                    TokenType::And => NodeValue::And(node, rhs),
                    TokenType::Or => NodeValue::Or(node, rhs),

                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                },
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn parse_if_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'if' をスキップ
        let mut condition = self.new_empty();
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::If(Box::new(*condition), Box::new(*body)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_for_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // for
        let var = self.current_token().unwrap().token_value().clone();
        self.next_token(); // var
        self.next_token(); // in

        //panic!("current: {:?}", self.current_token().unwrap().;
        let start_token = self.current_token().unwrap().token_value().clone();
        //panic!("{:?}",start_token);
        let iterator_node = if
        //self.current_token().unwrap().token_type() == TokenType::Range {
        self.peek_next_token(1).token_type() == TokenType::Range {
            self.next_token(); // skip ..
            self.next_token();
            let end_token = self.current_token().unwrap().token_value().clone();
            Box::new(Node::new(
                NodeValue::Range(
                    Box::new(Node::new(
                        NodeValue::Int(start_token.parse().unwrap()),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    Box::new(Node::new(
                        NodeValue::Int(end_token.parse().unwrap()),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                ),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else {
            Box::new(Node::new(
                NodeValue::Variable(start_token),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        };
        self.next_token(); // { をスキップ
        let body = self.parse_block()?;
        Ok(Box::new(Node::new(
            NodeValue::For(
                Box::new(Node::new(
                    NodeValue::Variable(var),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                iterator_node,
                body,
            ),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }
    fn parse_return_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // '->' をスキップ
        let return_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::ReturnType(return_type),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_while_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'while' をスキップ
        let mut condition = self.new_empty();
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::While(Box::new(*condition), Box::new(*body)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn factor(&mut self) -> Result<Box<Node>, String> {
        let mut token = self.current_token().unwrap().clone();
        let mut is_system = false;
        let mut node = Node::default();
        if token.token_type() == TokenType::AtSign {
            self.next_token();
            token = self.current_token().unwrap().clone();
            is_system = true;
        }

        match self.current_token().unwrap().token_type() {
            TokenType::MultiComment(content, (line, column)) => {
                self.next_token();
                node = Node::new(
                    NodeValue::MultiComment(content, (line, column)),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
            }
            TokenType::SingleComment(content, (line, column)) => {
                self.next_token();
                node = Node::new(
                    NodeValue::SingleComment(content, (line, column)),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
            }

            TokenType::DoubleQuote | TokenType::SingleQuote => {
                if let Ok(string) = token.token_value().parse::<String>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::String(string),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else {
                    return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected end of input_content, no closing DoubleQuote or SingleQuote found: {:?}",
                    self.current_token().unwrap()
                ));
                }
            }
            TokenType::Number => {
                if let Ok(number) = token.token_value().parse::<i64>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::Int(number),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else if let Ok(number) = token.token_value().parse::<f64>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::Float(number),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                }
            }
            TokenType::Ident => {
                if let Ok(bool_value) = token.token_value().parse::<bool>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::Bool(bool_value),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else {
                    self.next_token();
                    if self.current_token().unwrap().token_type() == TokenType::LeftParen {
                        node = *self.parse_function_call(token, is_system)?;
                    } else {
                        node = Node::new(
                            NodeValue::Variable(token.token_value().clone()),
                            None,
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        );
                    }
                }
            }
            TokenType::LeftParen => {
                self.next_token();
                let expr = self.expr()?;
                if self.current_token().unwrap().token_type() != TokenType::RightParen {
                    return Err(compile_error!(
                        "error",
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                        &self.input_path(),
                        &self.input_content(),
                        "no closing parenthesis in factor: {:?}",
                        self.current_token().unwrap()
                    ));
                } else {
                    self.next_token();
                    node = *self.expr()?;
                }
                return Ok(Box::new(node));
            }
            TokenType::LeftCurlyBrace => {
                node = *self.parse_block()?;
                return Ok(Box::new(node));
            }

            TokenType::LeftSquareBrace => {
                let data_type = self.new_empty();
                node = *self.parse_array(&data_type)?;
                return Ok(Box::new(node));
            }
            _ => {
                return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected token in factor: {:?}",
                    self.current_token()
                ));
            }
        }
        Ok(Box::new(node))
    }

    fn parse_block(&mut self) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // '{' をスキップ
        }
        let mut nodes = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
            //            info!("{:?}",self.current_token().unwrap().;

            if self.current_token().unwrap().token_type() == TokenType::Eof {
                return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected end of input, no closing curly brace found: {:?}",
                    self.current_token().unwrap()
                ));
            }
            let statements = self.parse_statement()?;
            nodes.extend(statements);
        }
        if self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
            return Err(compile_error!(
                "error",
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
                &self.input_path(),
                &self.input_content(),
                "no closing curly brace in block: {:?}",
                self.current_token().unwrap()
            ));
        } else {
            self.next_token(); // '}' をスキップ
            Ok(Box::new(Node::new(
                NodeValue::Block(nodes),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        }
    }
    fn parse_data_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // : をスキップ
        self.next_token(); // 変数名 をスキップ
        info!("current: {:?}", self.current_token().unwrap());
        let data_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(data_type),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }
    fn parse_array(&mut self, data_type: &Box<Node>) -> R<Box<Node>, String> {
        self.next_token(); // [ をスキップ
        let mut value_vec = vec![];
        while self.current_token().unwrap().token_type() != TokenType::RightSquareBrace {
            value_vec.push(self.expr()?);
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        //panic!("{:?}",value_vec);
        self.next_token(); // ] をスキップ
        Ok(Box::new(Node::new(
            NodeValue::Array(data_type.clone(), value_vec),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }
    fn parse_type_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // type
        let _type_name = self.current_token().unwrap().token_value().clone();
        self.next_token(); // =
        self.next_token(); // value
        let value_node = self.expr()?;
        Ok(Box::new(Node {
            node_value: NodeValue::TypeDeclaration(
                Box::new(Node::new(
                    NodeValue::Variable(_type_name),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                value_node,
            ),
            node_next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }
    fn parse_variable_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // let
        let mut is_mutable = false;
        if self.current_token().unwrap().token_value() == "mut"
            || self.current_token().unwrap().token_value() == "mutable"
        {
            self.next_token();
            is_mutable = true;
        }
        let var = self.current_token().unwrap().token_value().clone();
        let mut data_type = self.new_empty();
        let mut value_node = self.new_empty();
        if self.peek_next_token(1).token_type() == TokenType::Colon {
            data_type = self.parse_data_type()?;
        }
        self.next_token();
        //panic!("{:?}",self.current_token());

        let mut is_reference = false;
        if self.current_token().unwrap().token_type() == TokenType::Reference {
            is_reference = true;
            self.next_token();
        } else {
            value_node = self.expr()?;
        }
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
        }

        let mut is_local = false;
        let mut brace_count = 0;
        for i in (0..self.i).rev() {
            match self.tokens[i].token_type() {
                TokenType::LeftCurlyBrace => brace_count += 1,
                TokenType::RightCurlyBrace => brace_count -= 1,
                _ => {}
            }
            if brace_count > 0 {
                is_local = true;
                break;
            }
        }
        Ok(Box::new(Node {
            node_value: NodeValue::VariableDeclaration(
                Box::new(Node::new(
                    NodeValue::Variable(var),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                data_type,
                value_node,
                is_local,
                is_mutable,
                is_reference,
            ),
            node_next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }

    fn parse_assign_variable(&mut self) -> R<Box<Node>, String> {
        let var = self.current_token().unwrap().token_value().clone();
        let data_type = self.new_empty();
        let mut value_node = self.new_empty();
        let mut index = self.new_empty();

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftSquareBrace {
            self.next_token(); // [
            index = self.expr()?;

            self.next_token(); // ]
            self.next_token(); // =

            value_node = self.expr()?;
            Ok(Box::new(Node {
                node_value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(var),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                node_next: None,
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }))
        } else {
            self.next_token(); // =

            value_node = self.expr()?;

            Ok(Box::new(Node {
                node_value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(var),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                node_next: None,
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }))
        }
    }
    fn parse_return(&mut self) -> R<Box<Node>, String> {
        self.next_token();
        let mut ret_value = Box::new(Node::default());
        ret_value = self.expr()?;

        Ok(Box::new(Node {
            node_value: NodeValue::Return(ret_value),
            node_next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }
    fn parse_include(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // @
        self.next_token(); // include
        let include_file_path = self.current_token().unwrap().token_value().clone();
        let include_node = Node::new(
            NodeValue::Include(include_file_path),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        self.next_token();
        Ok(Box::new(include_node))
    }
    /*
        fn parse_single_statement(&mut self) -> R<Box<Node>, String> {
            let mut node: Option<Node> = None;
            if self.current_token().unwrap().token_value() == "callback" {
                node = Some(*self.parse_callback_function_definition()?);
            } else if self.current_token().unwrap().token_value() == "fn" {
                node = Some(*self.parse_function_definition()?);
            } else if self.current_token().unwrap().token_value() == "while" {
                node = Some(*self.parse_while_statement()?);
            } else if self.current_token().unwrap().token_value() == "if" {
                node = Some(*self.parse_if_statement()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Ident
                && self.current_token().unwrap().token_value() == "for"
                && self.peek_next_token(2).token_value() == "in"
            {
                node = Some(*self.parse_for_statement()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Ident
                && (self.current_token().unwrap().token_value() == "let"
                    || self.current_token().unwrap().token_value() == "var"
                    || self.current_token().unwrap().token_value() == "l"
                    || self.current_token().unwrap().token_value() == "v")
            {
                node = Some(*self.parse_variable_declaration()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Ident
                && self.peek_next_token(2).token_type() == TokenType::Equals
                && self.current_token().unwrap().token_value() == "type"
            {
                node = Some(*self.parse_type_declaration()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Ident
                && self.peek_next_token(1).token_type() == TokenType::Equals
                || self.current_token().unwrap().token_type() == TokenType::Ident
                    && self.peek_next_token(1).token_type() == TokenType::LeftSquareBrace
            {
                node = Some(*self.parse_assign_variable()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Ident
                && self.current_token().unwrap().token_value() == "return"
            {
                node = Some(*self.parse_return()?);
            } else if self.current_token().unwrap().token_type() == TokenType::AtSign
                && self.peek_next_token(1).token_type() == TokenType::Ident
                && self.peek_next_token(1).token_value() == "include"
            {
                node = Some(*self.parse_include()?);
            } else if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
                node = Some(*self.parse_block()?);
            } else if self.current_token().unwrap().token_type() == TokenType::Semi {
                self.is_statement = true;
                self.next_token();
            } else {
                self.is_statement = false;
                node = Some(*self.expr()?);
            }
               Ok(Box::new(node.unwrap_or_default()))

            //Ok(Box::new(node.unwrap()))
        }
    */

    fn parse_single_statement(&mut self) -> Option<R<Box<Node>, String>> {
        let result = if self.current_token().unwrap().token_value() == "callback" {
            self.parse_callback_function_definition()
        } else if self.current_token().unwrap().token_value() == "fn" {
            self.parse_function_definition()
        } else if self.current_token().unwrap().token_value() == "while" {
            self.parse_while_statement()
        } else if self.current_token().unwrap().token_value() == "if" {
            self.parse_if_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "for"
            && self.peek_next_token(2).token_value() == "in"
        {
            self.parse_for_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && (self.current_token().unwrap().token_value() == "let"
                || self.current_token().unwrap().token_value() == "var"
                || self.current_token().unwrap().token_value() == "l"
                || self.current_token().unwrap().token_value() == "v")
        {
            self.parse_variable_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(2).token_type() == TokenType::Equals
            && self.current_token().unwrap().token_value() == "type"
        {
            self.parse_type_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(1).token_type() == TokenType::Equals
            || self.current_token().unwrap().token_type() == TokenType::Ident
                && self.peek_next_token(1).token_type() == TokenType::LeftSquareBrace
        {
            self.parse_assign_variable()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "return"
        {
            self.parse_return()
        } else if self.current_token().unwrap().token_type() == TokenType::AtSign
            && self.peek_next_token(1).token_type() == TokenType::Ident
            && self.peek_next_token(1).token_value() == "include"
        {
            self.parse_include()
        } else if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.parse_block()
        } else if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
            self.next_token();
            return None; // 空ノードを生成せずにスキップ
        } else {
            self.is_statement = false;
            self.expr()
        };

        Some(result)
    }

    fn parse_statement(&mut self) -> R<Vec<Box<Node>>, String> {
        let mut nodes: Vec<Box<Node>> = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::Eof
            && self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace
        {
            if let Some(statement) = self.parse_single_statement() {
                nodes.push(statement?);
            }
        }
        Ok(nodes)
    }
    /*
        fn parse_statement(&mut self) -> R<Vec<Box<Node>>, String> {
            let mut nodes: Vec<Box<Node>> = Vec::new();
            while self.current_token().unwrap().token_type() != TokenType::Eof
                && self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace
            {
                let statement = self.parse_single_statement()?;
                nodes.push(statement);
            }
            Ok(nodes)
        }
    */
    pub fn parse(&mut self) -> R<Vec<Box<Node>>, String> {
        let mut nodes: Vec<Box<Node>> = Vec::new();
        nodes = self.parse_statement()?;
        Ok(nodes)
    }
}
