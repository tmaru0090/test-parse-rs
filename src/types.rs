use crate::parser::Node;

#[derive(PartialEq, Debug, Clone)]
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
    /*識別子*/
    Ident,
    /*その他*/
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
    Colon,                                     // :
    DoubleQuote,                               // "
    SingleQuote,                               // '
    SingleComment(String, (usize, usize)),     // "//"
    MultiComment(Vec<String>, (usize, usize)), // "/**/"
    RightArrow,                                // ->
    Eof,                                       // EOF
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
    Add(Box<Node>, Box<Node>),                 // 加算
    Sub(Box<Node>, Box<Node>),                 // 減算
    Mul(Box<Node>, Box<Node>),                 // 乗算
    Div(Box<Node>, Box<Node>),                 // 除算
    Increment(Box<Node>),                      // 増加
    Decrement(Box<Node>),                      // 減少
    AddAssign(Box<Node>, Box<Node>),           // 加算代入
    SubAssign(Box<Node>, Box<Node>),           // 減算代入
    MulAssign(Box<Node>, Box<Node>),           // 乗算代入
    DivAssign(Box<Node>, Box<Node>),           // 除算代入
    VariableDeclaration(Box<Node>, Box<Node>), //変数定義
    Assign(Box<Node>, Box<Node>),              // 代入
    Block(Vec<Box<Node>>),
    Variable(String),                                    // 変数
    Number(i32),                                         // 数値
    String(String),                                      // 文字列
    Bool(bool),                                          // 真偽値
    Function(String, Vec<String>, Box<Node>, Box<Node>), // 関数定義
    ReturnType(Box<Node>),                               // 関数の戻り値の型
    DataType(Box<Node>),                                 // 変数の型
    Call(String, Vec<Node>),                             // 関数呼び出し
    Return(Box<Node>),                                   // リターン
    MultiComment(Vec<String>, (usize, usize)),           // 複数コメント
    SingleComment(String, (usize, usize)),               // 単一コメント
    StatementEnd,                                        // ステートメントの終わり
    Empty,
}
