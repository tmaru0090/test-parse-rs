use crate::parser::Node;
#[derive(PartialEq, Debug, Clone)]
pub enum TokenType {
    /*記号*/
    Add,              // +
    Sub,              // -
    Mul,              // x
    Div,              // ÷
    LeftParen,        // (
    RightParen,       // )
    RightCurlyBrace,  // {
    LeftCurlyBrace,   // }
    LeftSquareBrace,  // [
    RightSquareBrace, // ]
    Comma,            // ,
    Equals,           // =
    AtSign,           // @
    Semi,             // ;
    DoubleQuote,      // "
    SingleQuote,      // '
    /*識別子*/
    Ident,
    /*その他*/
    Eof, // トークンの終わり
}

#[derive(PartialEq, Debug, Clone)]
pub enum NodeType {
    /*記号*/
    Add, // +
    Sub, // -
    Mul, // x
    Div, // ÷
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
