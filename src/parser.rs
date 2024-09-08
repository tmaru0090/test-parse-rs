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
    pub fn new_number(value: i32) -> Box<Node> {
        let node = Node::new(NodeType::Number(value), None);
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

    pub fn new_empty() -> Box<Node> {
        let node = Node::new(NodeType::Empty, None);
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
        let mut ret_value = Parser::<'_>::new_empty(); // 戻り値の初期値を指定
        if let NodeType::Block(ref nodes) = body.node_value() {
            if let Some(last_node) = nodes.last() {
                if let NodeType::Return(ref value) = last_node.node_value() {
                    ret_value = value.clone();
                }
            }
        }
        log::info!("Return value node: {:?}", ret_value);
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
                ret_value,
            ),
            None,
        )))
    }
    fn parse_condition(&mut self) -> R<Box<Node>> {
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
                    TokenType::Eq => NodeType::Eq(node, rhs),
                    TokenType::Ne => NodeType::Ne(node, rhs),
                    TokenType::Lt => NodeType::Lt(node, rhs),
                    TokenType::Gt => NodeType::Gt(node, rhs),
                    TokenType::Le => NodeType::Le(node, rhs),
                    TokenType::Ge => NodeType::Ge(node, rhs),
                    TokenType::And => NodeType::And(node, rhs),
                    TokenType::Or => NodeType::Or(node, rhs),
                    _ => panic!("Unexpected token: {:?}", op),
                },
                None,
            ));
        }
        Ok(node)
    }
    fn parse_if_statement(&mut self) -> R<Box<Node>> {
        self.next_token(); // 'if' をスキップ
        let mut condition = Parser::<'_>::new_empty();
        if self.current_token().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeType::If(Box::new(*condition), Box::new(*body)),
            None,
        )))
    }
    fn factor(&mut self) -> R<Box<Node>> {
        let token = self.current_token().clone();
        match token.token_type() {
            TokenType::MultiComment(content, (line, column)) => {
                self.next_token();
                Ok(Box::new(Node::new(
                    NodeType::MultiComment(content, (line, column)),
                    None,
                )))
            }
            TokenType::SingleComment(content, (line, column)) => {
                self.next_token();
                Ok(Box::new(Node::new(
                    NodeType::SingleComment(content, (line, column)),
                    None,
                )))
            }

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
                if token.token_value() == "if" {
                    self.parse_if_statement()
                } else if token.token_value() == "fn" {
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
            // 変数代入文か代入文か
            if self.current_token().token_type() == TokenType::Ident
                && self.current_token().token_value() == "let"
                && self.peek_next_token(2).token_type() == TokenType::Equals
            {
                self.next_token(); // let
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
