use crate::node::Node;
use crate::tokenizer::*;
use crate::types::*;
use std::collections::HashMap;
use std::str::FromStr;

impl FromStr for VarType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(int_value) = s.parse::<i64>() {
            Ok(VarType::Int(int_value))
        } else {
            Ok(VarType::String(String::from(s)))
        }
    }
}
pub struct Parser<'a> {
    pub tokens: &'a Vec<Token>,
    pub pos: usize,
    pub variables: HashMap<String, VarType>, // VarType enumを直接使用
}
impl<'a> Parser<'a> {
    fn get_variables(&self) -> HashMap<String, VarType> {
        self.variables.clone()
    }
    pub fn new(tokens: &'a Vec<Token>) -> Parser<'a> {
        Parser {
            tokens,
            pos: 0,
            variables: HashMap::new(),
        }
    }
    pub fn current_tokens(&self) -> Token {
        self.tokens[self.pos].clone()
    }
    pub fn next_tokens(&mut self) {
        self.pos += 1;
    }
    pub fn peek_next(&self, current_pos: usize) -> Option<&Token> {
        self.tokens.get(current_pos + 1)
    }
    /*
    fn block(&mut self) -> Result<Box<Node>, String> {
        let mut statements = Vec::new();

        // 波かっこの開始を確認
        if self.current_tokens().token_type != TokenType::LBlockDelimiter {
            return Err("Expected '{' at the beginning of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        loop {
            if self.current_tokens().token_type == TokenType::RBlockDelimiter {
                break; // ブロックの終わりに到達したらループを抜ける
            }

            let statement = self.expr()?; // ステートメントを解析
            statements.push(statement);

            // セミコロンの有無を確認
            if self.current_tokens().token_type != TokenType::Semi {
                return Err("Expected ';' at the end of statement".to_string());
            }
            self.next_tokens(); // セミコロンを読み進める
        }

        // 波かっこの終了を確認
        if self.current_tokens().token_type != TokenType::RBlockDelimiter {
            return Err("Expected '}' at the end of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        Ok(Box::new(Node::new(
            NodeType::Block(statements),
            Vec::new(),
            "".to_string(),
        )))
    }
    */

    fn block(&mut self) -> Result<Box<Node>, String> {
        let mut statements = Vec::new();

        // 波かっこの開始を確認
        if self.current_tokens().token_type != TokenType::LBlockDelimiter {
            return Err("Expected '{' at the beginning of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        loop {
            if self.current_tokens().token_type == TokenType::RBlockDelimiter {
                break; // ブロックの終わりに到達したらループを抜ける
            }

            let statement = self.expr()?; // ステートメントを解析
            statements.push(statement.clone());

            // セミコロンの有無を確認
            if self.current_tokens().token_type != TokenType::Semi {
                return Err("Expected ';' at the end of statement".to_string());
            }
            self.next_tokens(); // セミコロンを読み進める

            // デバッグ出力
            println!("Parsed statement: {:?}", statement.clone());
        }

        // 波かっこの終了を確認
        if self.current_tokens().token_type != TokenType::RBlockDelimiter {
            return Err("Expected '}' at the end of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        Ok(Box::new(Node::new(
            NodeType::Block(statements),
            Vec::new(),
            "".to_string(),
        )))
    }
    /*
    fn block(&mut self) -> Result<Box<Node>, String> {
        let mut statements = Vec::new();
        // 新しいスコープを作成
        // 波かっこの開始を確認
        if self.current_tokens().token_type != TokenType::LBlockDelimiter {
            return Err("Expected '{' at the beginning of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        while self.current_tokens().token_type != TokenType::RBlockDelimiter {
             let statement = self.expr()?; // 波かっこの中身を解析
            statements.push(statement);
            // 式の最後にセミコロンがあるかどうかの確認
            if self.current_tokens().token_type != TokenType::Semi {
                return Err("Expected ';' at the end of statement".to_string());
            }
            self.next_tokens(); // セミコロンを読み進める
        }

        if self.current_tokens().token_type != TokenType::RBlockDelimiter {
            return Err("Expected '}' at the end of block".to_string());
        }
        self.next_tokens(); // 次のトークンに進む

        Ok(Box::new(Node::new(
            NodeType::Block(statements),
            Vec::new(),
            "".to_string(),
        )))
    }*/
    pub fn expr(&mut self) -> Result<Box<Node>, String> {
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
                    _ => unreachable!(),
                },
                vec![node, self.factor()?],
                current_token.value,
            ));
        }
        Ok(node)
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
            TokenType::Ident => {
                self.next_tokens();
                let ident = current_token.value.clone();
                Ok(Box::new(Node::new(
                    NodeType::Var(ident.clone(), VarType::Int(0)),
                    vec![],
                    ident,
                )))
            }
            TokenType::LParen => {
                self.next_tokens();
                let node = self.expr()?;
                if self.current_tokens().token_type != TokenType::RParen {
                    return Err(format!(
                        "Expected closing parenthesis ')' but found {:?}",
                        self.current_tokens()
                    ));
                }
                self.next_tokens();
                Ok(node)
            }

            TokenType::LetDecl => {
                self.next_tokens(); // LetDeclトークンを読み進める
                if let TokenType::Ident = self.current_tokens().token_type {
                    let var_name = self.current_tokens().value.clone();
                    // 式になるまでトークンを進める
                    self.next_tokens();
                    self.next_tokens();

                    // 式を評価して結果を取得
                    let expr_node = self.expr()?; // 式を評価

                    // 式の結果を取得して変数にセット
                    let result = self.eval(&expr_node)?;
                    self.variables.insert(var_name.clone(), result.clone());
                    Ok(Box::new(Node::new(
                        NodeType::Var(var_name.clone(), result),
                        vec![],
                        var_name,
                    )))
                } else {
                    Err("Expected identifier after 'int' declaration".to_string())
                }
            }
            TokenType::Semi => {
                //println!("{:?}",self.current_tokens());
                //self.next_tokens();
                Ok(Box::new(Node::new(NodeType::Semi, vec![], ";".to_string())))
            }

            TokenType::Eof => Ok(Box::new(Node::new(
                NodeType::Error,
                vec![],
                "Unexpected end of input".to_string(),
            ))),
            _ => Err(format!("Unexpected token: {:?}", current_token)),
        }
    }

    pub fn print_var(&self, node: &Node, index: usize) -> Result<(), String> {
        let var_name = match self.variables.iter().nth(index) {
            Some((name, _value)) => name.clone(),
            None => return Err("Variable index out of bounds".to_string()),
        };

        match self.variables.get(&var_name) {
            Some(value) => {
                println!("var name: {:?}  value: {:?}", var_name, value);
                Ok(())
            }
            None => Err("Variable not found".to_string()),
        }
    }
    // Parser内のeval関数を以下のように更新します
    pub fn eval(&self, node: &Node) -> Result<VarType, String> {
        match &node.node_type {
            NodeType::Add => {
                let temp_left = VarType::from(self.eval(&node.child[0])?);
                let temp_right = VarType::from(self.eval(&node.child[1])?);
                let left = match temp_left {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                let right = match temp_right {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                Ok(VarType::Int(left + right))
            }
            NodeType::Sub => {
                let temp_left = VarType::from(self.eval(&node.child[0])?);
                let temp_right = VarType::from(self.eval(&node.child[1])?);
                let left = match temp_left {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                let right = match temp_right {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                Ok(VarType::Int(left - right))
            }
            NodeType::Mul => {
                let temp_left = VarType::from(self.eval(&node.child[0])?);
                let temp_right = VarType::from(self.eval(&node.child[1])?);
                let left = match temp_left {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                let right = match temp_right {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                Ok(VarType::Int(left * right))
            }
            NodeType::Div => {
                let temp_left = VarType::from(self.eval(&node.child[0])?);
                let temp_right = VarType::from(self.eval(&node.child[1])?);
                let left = match temp_left {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                let right = match temp_right {
                    VarType::Int(int) => int,
                    _ => -1,
                };
                Ok(VarType::Int(left / right))
            }

            // 他のパターンもVarTypeに応じて返り値の型を変更します
            NodeType::Var(variable_name, _) => {
                if let Some(value) = self.variables.get(variable_name) {
                    Ok(value.clone()) // 変数名に対応する値を返す
                } else {
                    Err("Variable not found".to_string())
                }
            }
            NodeType::Num(expression) => expression
                .parse::<VarType>()
                .map_err(|_| "Invalid expression".to_string()),
            _ => Err("Invalid operation".to_string()),
        }
    }

    pub fn program(&mut self) -> Result<Vec<Box<Node>>, String> {
        let mut nodes: Vec<Box<Node>> = Vec::new();

        while self.current_tokens().token_type != TokenType::Eof {
            // 波かっこの開始をチェック
            if self.current_tokens().token_type == TokenType::LBlockDelimiter {
                let block_node = self.block()?; // 波かっこの解析
                nodes.push(block_node);
            } else {
                let expr_node = self.expr()?; // 通常の式の解析
                nodes.push(expr_node);
            }
        }
        Ok(nodes)
    }
}


