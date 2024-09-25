pub static RESERVED_WORDS: &[&str] = &[
    "if", "else", "while", "for", "break", "continue", "i32", "i64", "f32", "f64", "u32", "u64",
    "type", "let", "l", "var", "v", "fn", "mut", "loop", "=", "+", "++", "-", "--", "+=", "-=",
    "*", "*=", "/", "/=", "{", "}", "[", "]", "mod", "use", "bool", "struct", "enum", "%", "&",
    "&=", "|", "|=", "^", "~", "^=",
];

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(any(feature = "full", feature = "parser"))]
use crate::parser::syntax::Node;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum TokenType {
    Add,
    Sub,
    Mul,
    Div,
    Increment,
    Decrement,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    ShiftLeft,
    ShiftRight,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
    Ident,
    Number,
    Reference,
    LeftParen,
    RightParen,
    RightCurlyBrace,
    LeftCurlyBrace,
    LeftSquareBrace,
    RightSquareBrace,
    Conma,
    Equals,
    AtSign,
    Semi,
    Colon,
    DoubleQuote,
    SingleQuote,
    SingleComment(String, (usize, usize)),
    MultiComment(Vec<String>, (usize, usize)),
    RightArrow,
    Eof,
    Range,
    ScopeResolution,
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ControlFlow {
    If(Box<Node>, Box<Node>),
    Else(Box<Node>),
    ElseIf(Box<Node>, Box<Node>),
    Loop(Box<Node>),
    While(Box<Node>, Box<Node>),
    For(Box<Node>, Box<Node>, Box<Node>),
    Return(Box<Node>),
    Break,
    Continue,
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Eq(Box<Node>, Box<Node>),
    Ne(Box<Node>, Box<Node>),
    Lt(Box<Node>, Box<Node>),
    Gt(Box<Node>, Box<Node>),
    Le(Box<Node>, Box<Node>),
    Ge(Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Increment(Box<Node>),
    Decrement(Box<Node>),
    AddAssign(Box<Node>, Box<Node>),
    SubAssign(Box<Node>, Box<Node>),
    MulAssign(Box<Node>, Box<Node>),
    DivAssign(Box<Node>, Box<Node>),
    BitAnd(Box<Node>, Box<Node>),
    BitOr(Box<Node>, Box<Node>),
    BitXor(Box<Node>, Box<Node>),
    BitNot(Box<Node>),
    ShiftLeft(Box<Node>, Box<Node>),
    ShiftRight(Box<Node>, Box<Node>),
    BitAndAssign(Box<Node>, Box<Node>),
    BitOrAssign(Box<Node>, Box<Node>),
    BitXorAssign(Box<Node>, Box<Node>),
    ShiftLeftAssign(Box<Node>, Box<Node>),
    ShiftRightAssign(Box<Node>, Box<Node>),
    Range(Box<Node>, Box<Node>),
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit(()),
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Declaration {
    Variable(Box<Node>, Box<Node>, Box<Node>, bool, bool, bool),
    Struct(String, Vec<Box<Node>>),
    Impl(String, Vec<Box<Node>>),
    Function(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool),
    CallBackFunction(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool),
    Type(Box<Node>, Box<Node>),
    Array(Box<Node>, Vec<Box<Node>>), // 配列(型名,値)
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeValue {
    ControlFlow(ControlFlow),
    Operator(Operator),
    DataType(DataType),
    Declaration(Declaration),
    Assign(Box<Node>, Box<Node>, Box<Node>),
    Block(Vec<Box<Node>>),
    Variable(Box<Node>, String),
    Call(String, Vec<Node>, bool),
    MultiComment(Vec<String>, (usize, usize)),
    SingleComment(String, (usize, usize)),
    Include(String),
    Mod(String),
    ModDeclaration(String, Vec<Box<Node>>),
    Use(String, Box<Node>),
    EndStatement,
    Null,
    Unknown,
}

#[cfg(any(feature = "full", feature = "parser"))]
impl Default for NodeValue {
    fn default() -> Self {
        NodeValue::Null
    }
}

impl From<Box<Node>> for DataType {
    fn from(node: Box<Node>) -> Self {
        match *node {
            Node {
                value: NodeValue::Variable(_, ref name),
                ..
            } => DataType::String(name.clone()),
            _ => DataType::Unit(()), // 他のケースに対するデフォルト処理
        }
    }
}
