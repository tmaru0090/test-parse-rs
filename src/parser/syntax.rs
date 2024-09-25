use crate::compile_error;
use crate::error::*;
use crate::lexer::tokenizer::Token;
use crate::types::*;
use anyhow::{anyhow, Context, Result as R};
use log::{error, info, warn};
use property_rs::Property;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Property, Serialize, Deserialize)]
pub struct Node {
    #[property(get)]
    pub value: NodeValue,
    #[property(get, set)]
    pub next: Option<Box<Node>>,
    #[property(get)]
    pub line: usize,
    #[property(get)]
    pub column: usize,
    #[property(get)]
    pub is_statement: bool,
}
pub struct NodeIter<'a> {
    current: Option<&'a Node>,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.current {
            self.current = node.next.as_deref();
            Some(node)
        } else {
            None
        }
    }
}

impl Node {
    pub fn iter(&self) -> NodeIter {
        NodeIter {
            current: Some(self),
        }
    }
}
impl Default for Node {
    fn default() -> Self {
        Node {
            value: NodeValue::default(),
            next: None,
            line: 0,
            column: 0,
            is_statement: false,
        }
    }
}

impl Node {
    pub fn new(value: NodeValue, next: Option<Box<Node>>, line: usize, column: usize) -> Self {
        Node {
            value,
            next,
            line,
            column,
            is_statement: false,
        }
    }
    pub fn is_next(&self) -> bool {
        self.next.is_some()
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
    pub fn new_add(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Add(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_sub(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Sub(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_mul(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Mul(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_div(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Div(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_int(value: i64, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Int(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_float(value: f64, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Float(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_string(value: String, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::String(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_bool(value: bool, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Bool(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_unit(line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::DataType(DataType::Unit(())), None, line, column);
        Box::new(node)
    }

    pub fn new_include(include_file: String, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Include(include_file), None, line, column);
        Box::new(node)
    }

    pub fn new_function(
        func_name: String,
        args: Vec<(Box<Node>, String)>,
        body: Box<Node>,
        return_type: Box<Node>,
        is_system: bool,
        line: usize,
        column: usize,
    ) -> Box<Node> {
        let node = Node::new(
            NodeValue::Declaration(Declaration::Function(
                func_name,
                args,
                body,
                return_type,
                is_system,
            )),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_variable(name: String, expr: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Variable(Parser::<'a>::new_null(line, column), name.clone()),
            Some(expr),
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_return(expr: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::ControlFlow(ControlFlow::Return(expr)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_unknown(&self) -> Box<Node> {
        let node = Node::new(
            NodeValue::Unknown,
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_null(line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Null, None, line, column);
        Box::new(node)
    }

    pub fn new_block(block: Vec<Box<Node>>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Block(block), None, line, column);
        Box::new(node)
    }

    pub fn new_assign(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Assign(left, right, Box::new(Node::default())),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn from_parse(
        tokens: &Vec<Token>,
        input_path: &str,
        input_content: String,
    ) -> R<Box<Node>, String> {
        let mut parser = Parser::new(tokens, input_path, input_content);
        parser.parse()
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.i)
    }

    fn peek_next_token(&mut self, i: usize) -> Option<Token> {
        if (self.i + i) < self.tokens.len() {
            Some(self.tokens[self.i + i].clone())
        } else {
            None
        }
    }
    fn previous_token(&mut self, i: usize) -> Option<Token> {
        if (self.i - 1) < self.tokens.len() {
            Some(self.tokens[self.i - i].clone())
        } else {
            None
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
                NodeValue::Operator(match op.token_type() {
                    TokenType::Mul => Operator::Mul(node, rhs),
                    TokenType::Div => Operator::Div(node, rhs),
                    TokenType::MulAssign => Operator::MulAssign(node, rhs),
                    TokenType::DivAssign => Operator::DivAssign(node, rhs),
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
                }),
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
                | TokenType::BitAnd
                | TokenType::BitOr
                | TokenType::BitXor
                | TokenType::ShiftLeft
                | TokenType::ShiftRight
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.term()?;
            node = Box::new(Node::new(
                NodeValue::Operator(match op.token_type() {
                    TokenType::Add => Operator::Add(node, rhs),
                    TokenType::Sub => Operator::Sub(node, rhs),
                    TokenType::AddAssign => Operator::AddAssign(node, rhs),
                    TokenType::SubAssign => Operator::SubAssign(node, rhs),
                    TokenType::Increment => Operator::Increment(node),
                    TokenType::Decrement => Operator::Decrement(node),
                    TokenType::BitAnd => Operator::BitAnd(node, rhs),
                    TokenType::BitOr => Operator::BitOr(node, rhs),
                    TokenType::BitXor => Operator::BitXor(node, rhs),
                    TokenType::ShiftLeft => Operator::ShiftLeft(node, rhs),
                    TokenType::ShiftRight => Operator::ShiftRight(node, rhs),
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
                }),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn factor(&mut self) -> R<Box<Node>, String> {
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
                        NodeValue::DataType(DataType::String(string)),
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
                        NodeValue::DataType(DataType::Int(number)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else if let Ok(number) = token.token_value().parse::<f64>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::DataType(DataType::Float(number)),
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
                        NodeValue::DataType(DataType::Bool(bool_value)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else {
                    self.next_token();
                    if self.current_token().unwrap().token_type() == TokenType::LeftParen {
                        node = *self.parse_function_call(token, is_system)?;
                    } else {
                        let mut data_type = Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        );

                        if self.current_token().unwrap().token_type() == TokenType::Colon {
                            self.next_token();
                            data_type = self.parse_data_type()?;
                        }
                        node = Node::new(
                            NodeValue::Variable(data_type, token.token_value().clone()),
                            None,
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        );
                    }
                }
            }
            TokenType::LeftParen => {
                self.next_token();
                node = *self.expr()?;
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
                }
                return Ok(Box::new(node));
            }

            TokenType::LeftCurlyBrace => {
                node = *self.parse_block()?;
                return Ok(Box::new(node));
            }
            TokenType::LeftSquareBrace => {
                let data_type = Parser::<'a>::new_null(
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
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
            value: NodeValue::Call(token.token_value().clone(), args, is_system),
            next: None,
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
            let mut return_type = Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            );
            while self.current_token().unwrap().token_type() != TokenType::RightParen {
                let arg = self.expr()?;
                let mut data_type = Parser::<'a>::new_null(
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
                if self.current_token().unwrap().token_type() == TokenType::Colon {
                    self.next_token(); // ':' をスキップ
                    data_type = self.expr()?;
                    data_type = Box::new(Node::new(
                        NodeValue::DataType(DataType::from(data_type)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    ));
                }
                let arg_name = match arg.value() {
                    NodeValue::Variable(_, ref name) => name.clone(),
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
                NodeValue::Declaration(Declaration::CallBackFunction(
                    name,
                    args,
                    Box::new(*body),
                    return_type,
                    is_system,
                )),
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
        let mut return_type = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        while self.current_token().unwrap().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            let mut data_type = Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            );
            if self.current_token().unwrap().token_type() == TokenType::Colon {
                self.next_token(); // ':' をスキップ
                data_type = self.expr()?;
                data_type = Box::new(Node::new(
                    NodeValue::DataType(DataType::from(data_type)),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                ));
            }
            let arg_name = match arg.value() {
                NodeValue::Variable(_, ref name) => name.clone(),
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
            NodeValue::Declaration(Declaration::Function(
                name,
                args,
                Box::new(*body),
                return_type,
                is_system,
            )),
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
                    TokenType::Eq => NodeValue::Operator(Operator::Eq(node, rhs)),
                    TokenType::Ne => NodeValue::Operator(Operator::Ne(node, rhs)),
                    TokenType::Lt => NodeValue::Operator(Operator::Lt(node, rhs)),
                    TokenType::Gt => NodeValue::Operator(Operator::Gt(node, rhs)),
                    TokenType::Le => NodeValue::Operator(Operator::Le(node, rhs)),
                    TokenType::Ge => NodeValue::Operator(Operator::Ge(node, rhs)),
                    TokenType::And => NodeValue::Operator(Operator::And(node, rhs)),
                    TokenType::Or => NodeValue::Operator(Operator::Or(node, rhs)),

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

    fn parse_loop_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'loop' をスキップ
        self.next_token(); // { をスキップ

        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Loop(body)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_break(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // break
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Break),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_continue(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // continue
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Continue),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_if_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'if' をスキップ
        let mut condition = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析

        let mut if_node = Node {
            value: NodeValue::ControlFlow(ControlFlow::If(Box::new(*condition), Box::new(*body))),
            next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: true,
        };

        // 'else' または 'else if' の処理
        if let Some(next_token) = self.peek_next_token(1) {
            if next_token.token_value() == "else" {
                self.next_token(); // 'else' をスキップ
                if let Some(next_token) = self.peek_next_token(1) {
                    if next_token.token_value() == "if" {
                        // 'else if' の処理
                        self.next_token(); // 'if' をスキップ
                        let else_if_node = self.parse_if_statement()?;
                        if_node.next = Some(else_if_node);
                    } else {
                        // 'else' の処理
                        self.next_token(); // { をスキップ
                        let else_body = self.parse_block()?;
                        let else_node = Node {
                            value: NodeValue::ControlFlow(ControlFlow::Else(Box::new(*else_body))),
                            next: None,
                            line: self.current_token().unwrap().line(),
                            column: self.current_token().unwrap().column(),
                            is_statement: true,
                        };
                        if_node.next = Some(Box::new(else_node));
                    }
                }
            }
        }

        Ok(Box::new(if_node))
    }

    fn parse_for_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // for
        let var = self.current_token().unwrap().token_value().clone();
        self.next_token(); // var
        self.next_token(); // in

        let start_token = self.current_token().unwrap().token_value().clone();
        let iterator_node = if self.peek_next_token(1).unwrap().token_type() == TokenType::Range {
            self.next_token(); // skip ..
            self.next_token();
            let end_token = self.current_token().unwrap().token_value().clone();
            Box::new(Node::new(
                NodeValue::Operator(Operator::Range(
                    Box::new(Node::new(
                        NodeValue::DataType(DataType::Int(start_token.parse().unwrap())),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    Box::new(Node::new(
                        NodeValue::DataType(DataType::Int(end_token.parse().unwrap())),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                )),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else {
            Box::new(Node::new(
                NodeValue::Variable(
                    Parser::<'a>::new_null(
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    ),
                    start_token,
                ),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        };
        self.next_token(); // { をスキップ
        let body = self.parse_block()?;
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::For(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        var,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                iterator_node,
                body,
            )),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_return_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // '->' をスキップ
        let return_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(DataType::from(return_type)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_while_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'while' をスキップ
        let mut condition = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::While(Box::new(*condition), Box::new(*body))),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_block(&mut self) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // '{' をスキップ
        }
        let mut nodes = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
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
            nodes.push(statements);
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
        if self.peek_next_token(1).unwrap().token_type() != TokenType::Eof
            && self.peek_next_token(1).unwrap().token_type() != TokenType::Conma
        {
            self.next_token(); // 変数名 をスキップ
        }
        info!("current: {:?}", self.current_token().unwrap());
        let data_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(DataType::from(data_type)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_type_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token();
        let _type_name = self.current_token().unwrap().token_value().clone();
        self.next_token();
        self.next_token();
        let value_node = self.expr()?;
        Ok(Box::new(Node {
            value: NodeValue::Declaration(Declaration::Type(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        _type_name,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                value_node,
            )),
            next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }

    fn parse_variable_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token();
        let mut is_mutable = false;
        if let Some(token) = self.current_token() {
            if token.token_value() == "mut" || token.token_value() == "mutable" {
                self.next_token();
                is_mutable = true;
            }
        }
        let var = self.current_token().unwrap().token_value().clone();
        let mut data_type = Box::new(Node::new(
            NodeValue::DataType(DataType::from(Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        ));
        let mut value_node = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.peek_next_token(1).unwrap().token_type() == TokenType::Colon {
            self.next_token();
            data_type = self.parse_data_type()?;
        }
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
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
            return Ok(Box::new(Node {
                value: NodeValue::Declaration(Declaration::Variable(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    data_type,
                    value_node,
                    is_local,
                    is_mutable,
                    false,
                )),
                next: None,
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }));
        }
        self.next_token();
        if self.current_token().unwrap().token_type() == TokenType::Equals {
            self.next_token();
        }
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
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
            return Ok(Box::new(Node {
                value: NodeValue::Declaration(Declaration::Variable(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    data_type,
                    value_node,
                    is_local,
                    is_mutable,
                    false,
                )),
                next: None,
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }));
        }
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
            value: NodeValue::Declaration(Declaration::Variable(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        var,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                data_type,
                value_node,
                is_local,
                is_mutable,
                is_reference,
            )),
            next: None,
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
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
        self.next_token(); // ] をスキップ
        Ok(Box::new(Node::new(
            NodeValue::Declaration(Declaration::Array(data_type.clone(), value_vec)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_assign_variable(&mut self) -> R<Box<Node>, String> {
        let var = self.current_token().unwrap().token_value().clone();
        let data_type = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        let mut value_node = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        let mut index = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftSquareBrace {
            self.next_token(); // [
            index = self.expr()?;

            self.next_token(); // ]
            self.next_token(); // =

            value_node = self.expr()?;
            Ok(Box::new(Node {
                value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                next: None,
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }))
        } else {
            self.next_token(); // =

            value_node = self.expr()?;

            Ok(Box::new(Node {
                value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                next: None,
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
            value: NodeValue::ControlFlow(ControlFlow::Return(ret_value)),
            next: None,
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

    fn parse_struct_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // struct
        let var = self.current_token().unwrap().token_value().clone();
        let mut member: Vec<Box<Node>> = Vec::new();

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // {
            while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
                let member_value = self.expr()?;
                member.push(member_value);
                if self.current_token().unwrap().token_type() == TokenType::Conma {
                    self.next_token(); // ',' をスキップ
                }
            }
            self.next_token(); // }
            Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::Struct(var.clone(), member.clone())),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        } else {
            if self.current_token().unwrap().token_type() == TokenType::Semi {
                self.is_statement = true;
            }
            Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::Struct(
                    var.clone(),
                    vec![Parser::<'a>::new_null(
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )],
                )),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        }
    }

    fn parse_single_statement(&mut self) -> Option<R<Box<Node>, String>> {
        let result = if self.current_token().unwrap().token_value() == "callback" {
            self.parse_callback_function_definition()
        } else if self.current_token().unwrap().token_value() == "struct" {
            self.parse_struct_definition()
        } else if self.current_token().unwrap().token_value() == "fn" {
            self.parse_function_definition()
        } else if self.current_token().unwrap().token_value() == "while" {
            self.parse_while_statement()
        } else if self.current_token().unwrap().token_value() == "if" {
            self.parse_if_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "for"
            && self.peek_next_token(2).unwrap().token_value() == "in"
        {
            self.parse_for_statement()
        } else if self.current_token().unwrap().token_value() == "loop" {
            self.parse_loop_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && (self.current_token().unwrap().token_value() == "let"
                || self.current_token().unwrap().token_value() == "var"
                || self.current_token().unwrap().token_value() == "l"
                || self.current_token().unwrap().token_value() == "v")
        {
            self.parse_variable_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(2).unwrap().token_type() == TokenType::Equals
            && self.current_token().unwrap().token_value() == "type"
        {
            self.parse_type_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(1).unwrap().token_type() == TokenType::Equals
            || self.current_token().unwrap().token_type() == TokenType::Ident
                && self.peek_next_token(1).unwrap().token_type() == TokenType::LeftSquareBrace
        {
            self.parse_assign_variable()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "return"
        {
            self.parse_return()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "break"
        {
            self.parse_break()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "continue"
        {
            self.parse_continue()
        } else if self.current_token().unwrap().token_type() == TokenType::AtSign
            && self.peek_next_token(1).unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(1).unwrap().token_value() == "include"
        {
            self.parse_include()
        } else if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.parse_block()
        } else if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
            self.next_token();
            return Some(Ok(Box::new(Node::new(
                NodeValue::EndStatement,
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))); // ステートメントを終了
        } else {
            self.is_statement = false;
            self.expr()
        };

        Some(result)
    }

    fn parse_statement(&mut self) -> R<Box<Node>, String> {
        self.parse_statement_recursive(None)
    }

    fn parse_statement_recursive(
        &mut self,
        previous_node: Option<Box<Node>>,
    ) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::Eof
            || self.current_token().unwrap().token_type() == TokenType::RightCurlyBrace
        {
            return previous_node.ok_or("No statements found".to_string());
        }

        if let Some(statement) = self.parse_single_statement() {
            let node = statement?;
            if let Some(mut prev) = previous_node {
                // ここで新しいノードを追加する
                let mut current = prev.as_mut();
                while current.next.is_some() {
                    current = current.next.as_mut().unwrap();
                }
                current.set_next(Some(node.clone()));
                self.parse_statement_recursive(Some(prev))
            } else {
                self.parse_statement_recursive(Some(node))
            }
        } else {
            Err("Failed to parse statement".to_string())
        }
    }
    pub fn parse(&mut self) -> R<Box<Node>, String> {
        self.parse_statement()
    }
}
