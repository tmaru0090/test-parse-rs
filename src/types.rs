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

    Add(Box<Node>, Box<Node>), // 加算
    Sub(Box<Node>, Box<Node>), // 減算
    Mul(Box<Node>, Box<Node>), // 乗算
    Div(Box<Node>, Box<Node>), // 除算

    Assign(Box<Node>, Box<Node>), // 代入
    Block(Vec<Node>),
    Variable(String),                         // 変数
    Number(i32),                              // 数値
    String(String),                           // 文字列
    Function(String, Vec<String>, Box<Node>), // 関数定義
    Call(String, Vec<Node>),                  // 関数呼び出し
    Return(Box<Node>),                        // リターン
    MultiComment(Vec<String>),                // 複数コメント
    SingleComment(String),                    // 単一コメント
}
