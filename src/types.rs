use crate::parser::Node;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
}
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeValue {
    /*制御構造文*/
    If(Box<Node>, Box<Node>), // If(条件,ボディ)
    Else(Box<Node>),
    ElseIf(Box<Node>, Box<Node>),         // ElseIf(条件,ボディ)
    Loop(Box<Node>),                      // Loop(ボディ)
    While(Box<Node>, Box<Node>),          // ElseIf(条件,ボディ)
    For(Box<Node>, Box<Node>, Box<Node>), // For(値、イテレータ|コレクション値|配列,ボディ)
    Break,                                // Break
    Continue,                             // Continue
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
    Increment(Box<Node>),      // 増加(左辺)
    Decrement(Box<Node>),      // 減少(左辺)

    AddAssign(Box<Node>, Box<Node>), // 加算代入(左辺,右辺)

    SubAssign(Box<Node>, Box<Node>), // 減算代入(左辺,右辺)

    MulAssign(Box<Node>, Box<Node>), // 乗算代入(左辺,右辺)

    DivAssign(Box<Node>, Box<Node>), // 除算代入(左辺,右辺)

    /*ビット演算用演算子*/
    BitAnd(Box<Node>, Box<Node>),           // ビットAND (左辺,右辺)
    BitOr(Box<Node>, Box<Node>),            // ビットOR (左辺,右辺)
    BitXor(Box<Node>, Box<Node>),           // ビットXOR (左辺,右辺)
    BitNot(Box<Node>),                      // ビットNOT (値)
    ShiftLeft(Box<Node>, Box<Node>),        // 左シフト (左辺,右辺)
    ShiftRight(Box<Node>, Box<Node>),       // 右シフト (左辺,右辺)
    BitAndAssign(Box<Node>, Box<Node>),     // ビットAND代入 (左辺,右辺)
    BitOrAssign(Box<Node>, Box<Node>),      // ビットOR代入 (左辺,右辺)
    BitXorAssign(Box<Node>, Box<Node>),     // ビットXOR代入 (左辺,右辺)
    ShiftLeftAssign(Box<Node>, Box<Node>),  // 左シフト代入 (左辺,右辺)
    ShiftRightAssign(Box<Node>, Box<Node>), // 右シフト代入 (左辺,右辺)
    VariableDeclaration(Box<Node>, Box<Node>, Box<Node>, bool, bool, bool), //変数定義(変数,型,右辺値,スコープフラグ,可変フラグ,参照フラグ)

    Assign(Box<Node>, Box<Node>, Box<Node>), // 代入(変数,右辺値,配列の場合のインデックス)
    Block(Vec<Box<Node>>),
    Variable(Box<Node>, String),    // 変数(型情報,変数名)
    Int(i64),                       // 数値(i64)
    Float(f64),                     // 浮動型少数の数値(f64)
    String(String),                 // 文字列(文字列)
    Bool(bool),                     // 真偽値(ブーリアン値)
    Unit(()),                       // Unit値(Void型)
    Struct(String, Vec<Box<Node>>), // 構造体定義(構造体名,メンバリスト)
    Function(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // 関数定義(関数名,(引数の型,引数名リスト),ボディ,戻り値,戻り値の型,システム関数フラグ)
    CallBackFunction(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // 関数定義(関数名,(引数の型,引数名リスト),ボディ,戻り値の型,システム関数フラグ)

    ReturnType(Box<Node>),                     // 関数の戻り値の型(戻り値の型)
    DataType(Box<Node>),                       // 変数の型
    Call(String, Vec<Node>, bool), // 関数呼び出し(関数名,引数名リスト,システム関数フラグ)
    Return(Box<Node>),             // リターン
    MultiComment(Vec<String>, (usize, usize)), // 複数コメント
    SingleComment(String, (usize, usize)), // 単一コメント
    Include(String),               // ファイル名
    TypeDeclaration(Box<Node>, Box<Node>), // 型定義(型名,型)
    Array(Box<Node>, Vec<Box<Node>>), // 配列(型名,値)
    Null,                          // 何もない値
    Unknown,                       // 異常値
}
impl Default for NodeValue {
    fn default() -> Self {
        NodeValue::Null
    }
}
pub static RESERVED_WORDS: &[&str] = &[
    "if", "else", "while", "for", "break", "continue", "i32", "i64", "f32", "f64", "u32", "u64",
    "type", "let", "l", "var", "v", "fn", "mut", "loop", "=", "+", "++", "-", "--", "+=", "-=",
    "*", "*=", "/", "/=", "{", "}", "[", "]",
];

pub enum DataType {
    Value(Value),
    Reference(Box<Value>),
}
