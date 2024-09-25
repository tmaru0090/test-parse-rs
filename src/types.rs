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

/*
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(any(feature = "full", feature = "parser"))]
use crate::parser::syntax::Node;
#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum TokenType {
    /*基本算術演算子*/
    Add,       // +
    Sub,       // -
    Mul,       // x
    Div,       // ÷
    Increment, // ++
    Decrement, // --
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=

    /*条件用演算子*/
    Eq,  // ==
    Ne,  // !=
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
    And, // &&
    Or,  // ||

    /*ビット演算用演算子*/
    BitAnd,           // &
    BitOr,            // |
    BitXor,           // ^
    BitNot,           // ~
    ShiftLeft,        // <<
    ShiftRight,       // >>
    BitAndAssign,     // &=
    BitOrAssign,      // |=
    BitXorAssign,     // ^=
    ShiftLeftAssign,  // <<=
    ShiftRightAssign, // >>=

    /*識別子*/
    Ident,
    Number,
    /*その他*/
    Reference,                                 // &
    LeftParen,                                 // (
    RightParen,                                // )
    RightCurlyBrace,                           // {
    LeftCurlyBrace,                            // }
    LeftSquareBrace,                           // [
    RightSquareBrace,                          // ]
    Conma,                                     // ,
    Equals,                                    // =
    AtSign,                                    // @
    Semi,                                      // ;
    Colon,                                     // :
    DoubleQuote,                               // "
    SingleQuote,                               // '
    SingleComment(String, (usize, usize)),     // "//"
    MultiComment(Vec<String>, (usize, usize)), // "/**/"
    RightArrow,                                // ->
    Eof,                                       // EOF
    Range,                                     // ..
    ScopeResolution,                           // ::
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeValue {
    /*制御構造文*/
If(Box<Node>, Box<Node>), // If(条件,ボディ)
    Else(Box<Node>),
ElseIf(Box<Node>, Box<Node>), // ElseIf(条件,ボディ)
Loop(Box<Node>), // Loop(ボディ)
While(Box<Node>, Box<Node>), // ElseIf(条件,ボディ)
For(Box<Node>, Box<Node>, Box<Node>), // For(値、イテレータ|コレクション値|配列,ボディ)
Break, // Break
Continue, // Continue
    Range(Box<Node>, Box<Node>),
/* 条件用演算子 */
Eq(Box<Node>, Box<Node>), // 等しい (左辺,右辺)
Ne(Box<Node>, Box<Node>), // 等しくない (左辺,右辺)

Lt(Box<Node>, Box<Node>), // 小なり (左辺,右辺)

Gt(Box<Node>, Box<Node>), // 大なり (左辺,右辺)

Le(Box<Node>, Box<Node>), // 以下 (左辺,右辺)

Ge(Box<Node>, Box<Node>), // 以上 (左辺,右辺)

And(Box<Node>, Box<Node>), // 論理積 (左辺,右辺)

Or(Box<Node>, Box<Node>), // 論理和 (左辺,右辺)

/*基本算術演算子*/
Add(Box<Node>, Box<Node>), // 加算(左辺,右辺)

Sub(Box<Node>, Box<Node>), // 減算(左辺,右辺)

Mul(Box<Node>, Box<Node>), // 乗算(左辺,右辺)

Div(Box<Node>, Box<Node>), // 除算(左辺,右辺)
Increment(Box<Node>), // 増加(左辺)
Decrement(Box<Node>), // 減少(左辺)

AddAssign(Box<Node>, Box<Node>), // 加算代入(左辺,右辺)

SubAssign(Box<Node>, Box<Node>), // 減算代入(左辺,右辺)

MulAssign(Box<Node>, Box<Node>), // 乗算代入(左辺,右辺)

DivAssign(Box<Node>, Box<Node>), // 除算代入(左辺,右辺)

/*ビット演算用演算子*/
BitAnd(Box<Node>, Box<Node>), // ビットAND (左辺,右辺)
BitOr(Box<Node>, Box<Node>), // ビットOR (左辺,右辺)
BitXor(Box<Node>, Box<Node>), // ビットXOR (左辺,右辺)
BitNot(Box<Node>), // ビットNOT (値)
ShiftLeft(Box<Node>, Box<Node>), // 左シフト (左辺,右辺)
ShiftRight(Box<Node>, Box<Node>), // 右シフト (左辺,右辺)
BitAndAssign(Box<Node>, Box<Node>), // ビットAND代入 (左辺,右辺)
BitOrAssign(Box<Node>, Box<Node>), // ビットOR代入 (左辺,右辺)
BitXorAssign(Box<Node>, Box<Node>), // ビットXOR代入 (左辺,右辺)
ShiftLeftAssign(Box<Node>, Box<Node>), // 左シフト代入 (左辺,右辺)
ShiftRightAssign(Box<Node>, Box<Node>), // 右シフト代入 (左辺,右辺)
VariableDeclaration(Box<Node>, Box<Node>, Box<Node>, bool, bool, bool), //変数定義(変数,型,右辺値,スコープフラグ,可変フラグ,参照フラグ)

Assign(Box<Node>, Box<Node>, Box<Node>), // 代入(変数,右辺値,配列の場合のインデックス)
    Block(Vec<Box<Node>>),
Variable(Box<Node>, String), // 変数(型情報,変数名)
Int(i64), // 数値(i64)
Float(f64), // 浮動型少数の数値(f64)
String(String), // 文字列(文字列)
Bool(bool), // 真偽値(ブーリアン値)
Unit(()), // Unit値(Void型)
Struct(String, Vec<Box<Node>>), // 構造体定義(構造体名,メンバリスト)
Impl(String, Vec<Box<Node>>), // 構造体実装(定義済み構造体名,メンバ関数リスト)
Function(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // 関数定義(関数名,(引数の型,引数名リスト),ボディ,戻り値,戻り値の型,システム関数フラグ)
CallBackFunction(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // 関数定義(関数名,(引数の型,引数名リスト),ボディ,戻り値の型,システム関数フラグ)

ReturnType(Box<Node>), // 関数の戻り値の型(戻り値の型)
DataType(Box<Node>), // 変数の型
Call(String, Vec<Node>, bool), // 関数呼び出し(関数名,引数名リスト,システム関数フラグ)
Return(Box<Node>), // リターン
MultiComment(Vec<String>, (usize, usize)), // 複数コメント
SingleComment(String, (usize, usize)), // 単一コメント
Include(String), // ファイルの全体コピー(ファイル名)
Mod(String), // モジュール宣言(モジュール名)
ModDeclaration(String, Vec<Box<Node>>), // モジュール定義(モジュール名,ボディ)
Use(String, Box<Node>), // モジュールのインポート(モジュール名,インポートモジュール)
TypeDeclaration(Box<Node>, Box<Node>), // 型定義(型名,型)
Array(Box<Node>, Vec<Box<Node>>), // 配列(型名,値)
Null, // 何もない値
Unknown, // 異常値
}

#[cfg(any(feature = "full", feature = "parser"))]
impl Default for NodeValue {
    fn default() -> Self {
        NodeValue::Null
    }
}
*/
