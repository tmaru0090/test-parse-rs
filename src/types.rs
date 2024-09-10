use crate::parser::Node;

#[derive(PartialEq, Debug, Clone)]
pub enum TokenType {
    /*記号全般*/
    Add,                                       // +
    Sub,                                       // -
    Mul,                                       // x
    Div,                                       // ÷
    LeftParen,                                 // (
    RightParen,                                // )
    RightCurlyBrace,                           // {
    LeftCurlyBrace,                            // }
    LeftSquareBrace,                           // [
    RightSquareBrace,                          // ]
    Comma,                                     // ,
    Equals,                                    // =
    AtSign,                                    // @
    Semi,                                      // ;
    DoubleQuote,                               // "
    SingleQuote,                               // '
    SingleComment(String, (usize, usize)),     // "//"
    MultiComment(Vec<String>, (usize, usize)), // "/**/"
    /*条件用演算子*/
    Eq,  // ==
    Ne,  // !=
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
    And, // &&
    Or,  // ||
    /*識別子*/
    Ident,
    /*その他*/
    Eof, // トークンの終わり
}

#[derive(PartialEq, Debug, Clone)]
pub enum NodeValue {
    /*制御構造文*/
    If(Box<Node>, Box<Node>),
    Else(Box<Node>),
    ElseIf(Box<Node>, Box<Node>),
    While(Box<Node>, Box<Node>),
    For(Box<Node>, Box<Node>),
    /* 条件用演算子 */
    Eq(Box<Node>, Box<Node>),  // 等しい (==)
    Ne(Box<Node>, Box<Node>),  // 等しくない (!=)
    Lt(Box<Node>, Box<Node>),  // 小なり (<)
    Gt(Box<Node>, Box<Node>),  // 大なり (>)
    Le(Box<Node>, Box<Node>),  // 以下 (<=)
    Ge(Box<Node>, Box<Node>),  // 以上 (>=)
    And(Box<Node>, Box<Node>), // 論理積 (&&)
    Or(Box<Node>, Box<Node>),  // 論理和 (||)
    /*基本算術演算子*/
    Add(Box<Node>, Box<Node>), // 加算
    Sub(Box<Node>, Box<Node>), // 減算
    Mul(Box<Node>, Box<Node>), // 乗算
    Div(Box<Node>, Box<Node>), // 除算

    Assign(Box<Node>, Box<Node>), // 代入
    Block(Vec<Node>),
    Variable(String),                                    // 変数
    Number(i32),                                         // 数値
    String(String),                                      // 文字列
    Bool(bool),                                          // 真偽値
    Function(String, Vec<String>, Box<Node>, Box<Node>), // 関数定義
    Call(String, Vec<Node>),                             // 関数呼び出し
    Return(Box<Node>),                                   // リターン
    MultiComment(Vec<String>, (usize, usize)),           // 複数コメント
    SingleComment(String, (usize, usize)),               // 単一コメント
    Empty,
}
