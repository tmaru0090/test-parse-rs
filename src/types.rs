use crate::node::Node;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TokenType {
    LBlockDelimiter, // {
    RBlockDelimiter, // }
    Char,            // ''
    String,          // ""
    Int,             // 整数値
    LetDecl,         // let宣言
    Ident,           // 識別子
    Add,             // +
    Sub,             // -
    Mul,             // *
    Div,             // /
    LParen,          // (
    RParen,          // )
    Assign,          // =
    Semi,            // ;
    Eof,             // トークンの終わり
    Error,           // エラー時
}
#[derive(PartialEq, Debug, Clone)]
pub enum NodeType {
    Var(String, VarType), // 変数名と値のペアを保持する場合
    VarDecl,              // 変数宣言
    VarAssign,            // 変数代入
    Add,                  // +
    Sub,                  // -
    Mul,                  // *
    Div,                  // /
    Num(String),          // 値
    Error,                // エラー
    Semi,                 // 式の終わり
    Block(Vec<Box<Node>>),
}
#[derive(Clone, Debug, PartialEq)]
pub enum VarType {
    Int(i64),
    String(String),
    Bool(bool),
    Float(f64),
}
