use crate::custom_compile_error;
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
}
impl Default for Node {
    fn default() -> Self {
        Node {
            node_value: NodeValue::default(),
            node_next: None,
            line: 0,
            column: 0,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parser<'a> {
    input_content: String,
    input_path: String,
    tokens: &'a Vec<Token>,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>, input_path: String, input_content: String) -> Self {
        Parser {
            tokens,
            i: 0,
            input_path,
            input_content,
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
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }
    pub fn new_sub(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Sub(left, right),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }
    pub fn new_mul(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Mul(left, right),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );

        Box::new(node)
    }
    pub fn new_div(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Div(left, right),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );

        Box::new(node)
    }
    pub fn new_int(&self, value: i64) -> Box<Node> {
        let node = Node::new(
            NodeValue::Int(value),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }
    pub fn new_float(&self, value: f64) -> Box<Node> {
        let node = Node::new(
            NodeValue::Float(value),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }

    pub fn new_variable(&self, name: String, expr: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Variable(name.clone()),
            Some(expr),
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }

    pub fn new_return(&self, expr: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Return(expr),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }

    pub fn new_empty(&self) -> Box<Node> {
        let node = Node::new(
            NodeValue::Empty,
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }
    pub fn new_block(&self, block: Vec<Box<Node>>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Block(block),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
        Box::new(node)
    }

    pub fn new_assign(&self, left: Box<Node>, right: Box<Node>) -> Box<Node> {
        let node = Node::new(
            NodeValue::Assign(left, right),
            None,
            self.current_token().line(),
            self.current_token().column(),
        );
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
    fn term(&mut self) -> R<Box<Node>, String> {
        let mut node = self.factor()?;
        while matches!(
            self.current_token().token_type(),
            TokenType::Mul | TokenType::Div | TokenType::MulAssign | TokenType::DivAssign
        ) {
            let op = self.current_token().clone();
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
                        custom_compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token(),
                        )
                    ),
                },
                None,
                self.current_token().line(),
                self.current_token().column(),
            ));
        }
        Ok(node)
    }

    fn expr(&mut self) -> R<Box<Node>, String> {
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
                    TokenType::Add => NodeValue::Add(node, rhs),
                    TokenType::Sub => NodeValue::Sub(node, rhs),
                    TokenType::AddAssign => NodeValue::AddAssign(node, rhs),
                    TokenType::SubAssign => NodeValue::SubAssign(node, rhs),
                    _ => panic!(
                        "{}",
                        custom_compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token(),
                        )
                    ),
                },
                None,
                self.current_token().line(),
                self.current_token().column(),
            ));
        }
        Ok(node)
    }
    fn parse_function_call(&mut self, token: Token) -> R<Box<Node>, String> {
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        while self.current_token().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ
        Ok(Box::new(Node::new(
            NodeValue::Call(token.token_value().clone(), args),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }

    fn parse_function_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'fn' をスキップ
        let name = self.current_token().token_value().clone();
        self.next_token(); // 関数名をスキップ
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        let mut return_type = self.new_empty();
        while self.current_token().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ
        if self.current_token().token_type() == TokenType::RightArrow {
            return_type = self.parse_return_type()?;
        }
        let body = self.parse_block()?; // ブロックの解析

        let mut ret_value = self.new_empty(); // 戻り値の初期値を指定
        if let NodeValue::Block(ref nodes) = body.node_value() {
            if let Some(last_node) = nodes.last() {
                if let NodeValue::Return(ref value) = last_node.node_value() {
                    ret_value = value.clone();
                }
            }
        }
        Ok(Box::new(Node::new(
            NodeValue::Function(
                name,
                args.iter()
                    .map(|arg| match arg.node_value() {
                        NodeValue::Variable(ref name) => name.clone(),
                        _ => "".to_string(),
                    })
                    .collect(),
                Box::new(*body),
                ret_value,
                return_type,
            ),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }
    fn parse_condition(&mut self) -> R<Box<Node>, String> {
        let mut node = self.expr()?; // 基本の式を解析

        while matches!(
            self.current_token().token_type(),
            TokenType::Eq
                | TokenType::Ne
                | TokenType::Lt
                | TokenType::Gt
                | TokenType::Le
                | TokenType::Ge
                | TokenType::And
                | TokenType::Or
        ) {
            let op = self.current_token().clone();
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
                        custom_compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token(),
                        )
                    ),
                },
                None,
                self.current_token().line(),
                self.current_token().column(),
            ));
        }
        Ok(node)
    }
    fn parse_if_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'if' をスキップ
        let mut condition = self.new_empty();
        if self.current_token().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::If(Box::new(*condition), Box::new(*body)),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }
    fn parse_for_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // for
        let var = self.current_token().token_value().clone();
        self.next_token(); // var
        self.next_token(); // in
        let array = self.current_token().token_value().clone();
        self.next_token(); // { をスキップ
        let body = self.parse_block()?;
        Ok(Box::new(Node::new(
            NodeValue::For(
                Box::new(Node::new(
                    NodeValue::Variable(var),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                )),
                Box::new(Node::new(
                    NodeValue::Variable(array),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                )),
                body,
            ),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }

    fn parse_return_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // '->' をスキップ
        let return_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::ReturnType(return_type),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }

    fn parse_while_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'while' をスキップ
        let mut condition = self.new_empty();
        if self.current_token().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::While(Box::new(*condition), Box::new(*body)),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }

    fn factor(&mut self) -> R<Box<Node>, String> {
        let token = self.current_token().clone();
        //   info!("current_token: {:?}", token);
        match token.token_type() {
            TokenType::MultiComment(content, (line, column)) => {
                self.next_token();
                Ok(Box::new(Node::new(
                    NodeValue::MultiComment(content, (line, column)),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                )))
            }
            TokenType::SingleComment(content, (line, column)) => {
                self.next_token();
                Ok(Box::new(Node::new(
                    NodeValue::SingleComment(content, (line, column)),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                )))
            }

            TokenType::DoubleQuote | TokenType::SingleQuote => {
                if let Ok(string) = token.token_value().parse::<String>() {
                    self.next_token();
                    Ok(Box::new(Node::new(
                        NodeValue::String(string),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )))
                } else {
                    return Err(custom_compile_error!(
                        "error",
                        self.current_token().line(),
                        self.current_token().column(),

                        &self.input_path(),
                        &self.input_content(),
                        "Unexpected end of input_content, no closing DoubleQuote or SingleQuote found: {:?}",
                        self.current_token(),
                    ));
                }
            }
            TokenType::Ident => {
                if token.token_value() == "if" {
                    self.parse_if_statement()
                } else if token.token_value() == "while" {
                    self.parse_while_statement()
                } else if token.token_value() == "fn" {
                    self.parse_function_definition()
                } else if let Ok(bool_value) = token.token_value().parse::<bool>() {
                    self.next_token();
                    Ok(Box::new(Node::new(
                        NodeValue::Bool(bool_value),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )))
                } else if let Ok(number) = token.token_value().parse::<i64>() {
                    self.next_token();
                    Ok(Box::new(Node::new(
                        NodeValue::Int(number),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )))
                } else if let Ok(number) = token.token_value().parse::<f64>() {
                    self.next_token();
                    Ok(Box::new(Node::new(
                        NodeValue::Float(number),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )))
                } else {
                    self.next_token();
                    if self.current_token().token_type() == TokenType::LeftParen {
                        self.parse_function_call(token)
                    } else {
                        Ok(Box::new(Node::new(
                            NodeValue::Variable(token.token_value().clone()),
                            None,
                            self.current_token().line(),
                            self.current_token().column(),
                        )))
                    }
                }
            }
            TokenType::LeftParen => {
                self.next_token();
                let expr = self.expr()?;
                if self.current_token().token_type() != TokenType::RightParen {
                    return Err(custom_compile_error!(
                        "error",
                        self.current_token().line(),
                        self.current_token().column(),
                        &self.input_path(),
                        &self.input_content(),
                        "no closing parenthesis in factor: {:?}",
                        self.current_token(),
                    ));
                } else {
                    self.next_token();
                    Ok(expr)
                }
            }
            TokenType::LeftCurlyBrace => self.parse_block(),

            _ => Err(custom_compile_error!(
                "error",
                self.current_token().line(),
                self.current_token().column(),
                &self.input_path(),
                &self.input_content(),
                "Unexpected token in factor: {:?}",
                self.current_token()
            )),
        }
    }
    fn parse_block(&mut self) -> R<Box<Node>, String> {
        if self.current_token().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // '{' をスキップ
        }
        let mut nodes = Vec::new();
        while self.current_token().token_type() != TokenType::RightCurlyBrace {
            if self.current_token().token_type() == TokenType::Eof {
                return Err(custom_compile_error!(
                    "error",
                    self.current_token().line(),
                    self.current_token().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected end of input_content, no closing curly brace found: {:?}",
                    self.current_token(),
                ));
            }

            let statements = self.parse_statement()?;
            nodes.extend(statements);
        }

        if self.current_token().token_type() != TokenType::RightCurlyBrace {
            return Err(custom_compile_error!(
                "error",
                self.current_token().line(),
                self.current_token().column(),
                &self.input_path(),
                &self.input_content(),
                "no closing curly brace in block: {:?}",
                self.current_token(),
            ));
        } else {
            self.next_token(); // '}' をスキップ
            Ok(Box::new(Node::new(
                NodeValue::Block(nodes),
                None,
                self.current_token().line(),
                self.current_token().column(),
            )))
        }
    }
    fn parse_data_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // : をスキップ
        self.next_token(); // 変数名 をスキップ
        info!("current: {:?}", self.current_token());
        let data_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(data_type),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }
    fn parse_array(&mut self, data_type: &Box<Node>) -> R<Box<Node>, String> {
        self.next_token(); // [ をスキップ
        let mut value_vec = vec![];
        while self.current_token().token_type() != TokenType::RightSquareBrace {
            value_vec.push(self.expr()?);
            if self.current_token().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ] をスキップ
        Ok(Box::new(Node::new(
            NodeValue::Array(data_type.clone(), value_vec),
            None,
            self.current_token().line(),
            self.current_token().column(),
        )))
    }
    fn parse_single_statement(&mut self) -> R<Box<Node>, String> {
        let node;
        if self.current_token().token_type() == TokenType::Ident
            && self.current_token().token_value() == "if"
        {
            node = *self.parse_if_statement()?;
        } else if self.current_token().token_type() == TokenType::Ident
            && self.current_token().token_value() == "for"
            && self.peek_next_token(2).token_value() == "in"
        {
            node = *self.parse_for_statement()?;
        } else if self.current_token().token_type() == TokenType::Ident
            && self.current_token().token_value() == "let"
            || self.current_token().token_value() == "var"
            || self.current_token().token_value() == "l"
            || self.current_token().token_value() == "v"
        {
            self.next_token();
            let mut is_mutable = false;
            if self.current_token().token_value() == "mut"
                || self.current_token().token_value() == "mutable"
            {
                self.next_token();
                is_mutable = true;
            }

            let var = self.current_token().token_value().clone();
            let mut data_type = self.new_empty();
            let mut value_node = self.new_empty();

            if self.peek_next_token(1).token_type() == TokenType::Colon {
                data_type = self.parse_data_type()?;
                self.next_token();
            }

            if self.peek_next_token(1).token_type() == TokenType::Equals {
                self.next_token();
                self.next_token();
            }
            let mut is_reference = false;
            if self.current_token().token_type() == TokenType::Reference {
                is_reference = true;
                self.next_token();
            }
            if self.peek_next_token(1).token_type() == TokenType::LeftSquareBrace
                || self.current_token().token_type() == TokenType::LeftSquareBrace
            {
                value_node = self.parse_array(&data_type)?;
            } else {
                value_node = self.expr()?;
            }

            // ローカルスコープフラグの初期化
            let mut is_local = false;

            // 現在のトークンがブロック内にあるか確認
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

            node = Node::new(
                NodeValue::VariableDeclaration(
                    Box::new(Node::new(
                        NodeValue::Variable(var),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    data_type,
                    value_node,
                    is_local,
                    is_mutable,
                    is_reference,
                ),
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::Ident
            && self.peek_next_token(2).token_type() == TokenType::Equals
            && self.current_token().token_value() == "type"
        {
            self.next_token();
            let _type_name = self.current_token().token_value().clone();
            self.next_token(); // =
            self.next_token(); // value
            let value_node = self.expr()?;
            node = Node::new(
                NodeValue::TypeDeclaration(
                    Box::new(Node::new(
                        NodeValue::Variable(_type_name),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    value_node,
                ),
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::Ident
            && self.peek_next_token(1).token_type() == TokenType::Equals
        {
            let var = self.current_token().token_value().clone();
            self.next_token();
            self.next_token();
            let value_node = self.expr()?;
            node = Node::new(
                NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(var),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    value_node,
                ),
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::Ident
            && matches!(
                self.peek_next_token(1).token_type(),
                TokenType::AddAssign
                    | TokenType::SubAssign
                    | TokenType::MulAssign
                    | TokenType::DivAssign
                    | TokenType::Increment
                    | TokenType::Decrement
            )
        {
            let var = self.current_token().token_value().clone();
            let op = self.peek_next_token(1).token_type().clone();
            self.next_token();
            self.next_token();
            let node_value = match op {
                TokenType::AddAssign => NodeValue::AddAssign(
                    Box::new(Node::new(
                        NodeValue::Variable(var.clone()),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    self.expr()?,
                ),
                TokenType::SubAssign => NodeValue::SubAssign(
                    Box::new(Node::new(
                        NodeValue::Variable(var.clone()),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    self.expr()?,
                ),
                TokenType::MulAssign => NodeValue::MulAssign(
                    Box::new(Node::new(
                        NodeValue::Variable(var.clone()),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    self.expr()?,
                ),
                TokenType::DivAssign => NodeValue::DivAssign(
                    Box::new(Node::new(
                        NodeValue::Variable(var.clone()),
                        None,
                        self.current_token().line(),
                        self.current_token().column(),
                    )),
                    self.expr()?,
                ),
                TokenType::Increment => NodeValue::Increment(Box::new(Node::new(
                    NodeValue::Variable(var.clone()),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                ))),
                TokenType::Decrement => NodeValue::Decrement(Box::new(Node::new(
                    NodeValue::Variable(var.clone()),
                    None,
                    self.current_token().line(),
                    self.current_token().column(),
                ))),
                _ => unreachable!(),
            };
            node = Node::new(
                node_value,
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::Ident
            && self.current_token().token_value() == "return"
        {
            self.next_token();
            let ret_value = self.expr()?;
            node = Node::new(
                NodeValue::Return(ret_value),
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::Ident
            && self.current_token().token_value() == "include"
        {
            self.next_token();
            let include_file_path = self.current_token().token_value().clone();
            node = Node::new(
                NodeValue::Include(include_file_path),
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else if self.current_token().token_type() == TokenType::LeftCurlyBrace {
            node = *self.parse_block()?;
        } else if self.current_token().token_type() == TokenType::Semi {
            self.next_token();
            node = Node::new(
                NodeValue::StatementEnd,
                None,
                self.current_token().line(),
                self.current_token().column(),
            );
        } else {
            node = *self.expr()?;
        }

        Ok(Box::new(node))
    }
    fn parse_statement(&mut self) -> R<Vec<Box<Node>>, String> {
        let mut nodes: Vec<Box<Node>> = Vec::new();
        while self.current_token().token_type() != TokenType::Eof
            && self.current_token().token_type() != TokenType::RightCurlyBrace
        {
            let statement = self.parse_single_statement()?;
            nodes.push(statement);
        }
        Ok(nodes)
    }

    pub fn parse(&mut self) -> R<Vec<Box<Node>>, String> {
        let mut nodes: Vec<Box<Node>> = Vec::new();
        nodes = self.parse_statement()?;
        Ok(nodes)
    }
}
