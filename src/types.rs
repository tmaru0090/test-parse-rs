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
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum NodeType {
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
#[derive(Clone, Debug)]
pub enum VarType {
    Int(i64),
    String(String),
    Bool(bool),
    Float(f64),
}
