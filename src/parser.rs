use crate::tokenizer::*;
use crate::types::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Error;
use std::str::FromStr;
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Node {
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
                    NodeType::Var(ident.clone()),
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
                    self.variables.insert(var_name.clone(), result);
                    Ok(Box::new(Node::new(
                        NodeType::Var(var_name.clone()),
                        vec![],
                        var_name,
                    )))
                } else {
                    Err("Expected identifier after 'int' declaration".to_string())
                }
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
            NodeType::Var(variable_name) => {
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
}

pub struct ScopeManager {
    pub scopes: Vec<HashMap<String, VarType>>, // スコープ毎の変数名と値の関連付けを管理するVec
}

impl ScopeManager {
    pub fn new() -> ScopeManager {
        ScopeManager {
            scopes: vec![HashMap::new()],
        } // 初期スコープを作成
    }

    fn create_scope(&mut self) {
        self.scopes.push(HashMap::new()); // 新しいスコープを作成して追加
    }

    fn destroy_scope(&mut self) {
        self.scopes.pop(); // 最後のスコープを削除
    }

    fn set_variable(&mut self, name: String, value: VarType) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value); // 現在のスコープに変数を追加
            Ok(())
        } else {
            Err("No scope exists".to_string())
        }
    }

    fn get_variable(&self, name: &str) -> Option<VarType> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone()); // スコープから変数を取
            }
        }
        None
    }
}

pub struct Decoder<'a> {
    pub parser: &'a Parser<'a>,
    pub scope_manager: &'a mut ScopeManager,
}

impl<'a> Decoder<'a> {
    pub fn new(parser: &'a Parser, scope_manager: &'a mut ScopeManager) -> Decoder<'a> {
        Decoder {
            parser,
            scope_manager,
        }
    }

    pub fn decode(&mut self, program: &Vec<Box<Node>>) -> Result<(), String> {
        // 今回は単純に宣言された変数のリストを表示
        for (index, node) in program.iter().enumerate() {
            match &node.node_type {
                NodeType::VarDecl => {
                    if let Some(node) = node.child.get(0) {
                        if let NodeType::Var(name) = &node.node_type {
                            let var_name = name;
                            let result = self.parser.eval(node)?;
                            self.scope_manager.set_variable(var_name.clone(), result)?;
                        }
                    }
                }
                _ => (),
            }
            self.parser.print_var(node, index)?;
        }

        Ok(())
    }
}
// ファイルから内容を取得
pub fn read_file(file_name: &str) -> Result<String, Error> {
    // ファイルを読み込む
    let contents = fs::read_to_string(file_name)?;
    // すべての式を評価して結果を表示

    Ok(contents)
}

// トークン化データからプログラムノードのリストを返す
pub fn program(parser: &mut Parser) -> Result<Vec<Box<Node>>, String> {
    let mut nodes: Vec<Box<Node>> = Vec::new();

    // トークン列をすべて処理する
    loop {
        let node = parser.expr()?;
        nodes.push(node);

        // 次のトークンがEOFかどうかチェック
        if parser.current_tokens().token_type == TokenType::Eof {
            break;
        }
    }
    Ok(nodes)
}
