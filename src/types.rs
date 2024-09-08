use crate::parser::Node;
#[derive(PartialEq, Debug, Clone)]
pub enum TokenType {
    /*記号全般*/
    Add,                       // +
    Sub,                       // -
    Mul,                       // x
    Div,                       // ÷
    LeftParen,                 // (
    RightParen,                // )
    RightCurlyBrace,           // {
    LeftCurlyBrace,            // }
    LeftSquareBrace,           // [
    RightSquareBrace,          // ]
    Comma,                     // ,
    Equals,                    // =
    AtSign,                    // @
    Semi,                      // ;
    DoubleQuote,               // "
    SingleQuote,               // '
    SingleComment(String),     // "//"
    MultiComment(Vec<String>), // "/**/"
    /*条件用演算子*/
    // ==
    // !=
    // <
    // >
    // <=
    // >=
    // &&
    // ||
    /*識別子*/
    Ident,
    /*その他*/
    Eof, // トークンの終わり
}

#[derive(PartialEq, Debug, Clone)]
pub enum NodeType {
    /*制御構造文*/
    If(Box<Node>, Box<Node>),
    Else(Box<Node>),
    ElseIf(Box<Node>, Box<Node>),
    For(Box<Node>, Box<Node>),

    /*記号全般*/
    Add(Box<Node>, Box<Node>), // +
    Sub(Box<Node>, Box<Node>), // -
    Mul(Box<Node>, Box<Node>), // x
    Div(Box<Node>, Box<Node>), // ÷
    /*変数*/
    Assign(Box<Node>, Box<Node>), // 代入
    Block(Vec<Node>),
    Variable(String), // 変数
    Number(i32),      // 数値
    String(String),   // 文字列
    Function(String, Vec<String>, Box<Node>),
    Call(String, Vec<Node>),
    Return(Box<Node>),
}
