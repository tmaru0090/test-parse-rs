use crate::parser::Node;

use serde::Serialize;
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
#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum NodeValue {
    /*制御構造文*/
    If(Box<Node>, Box<Node>), // If(条件,ボディ)
    Else(Box<Node>),
    ElseIf(Box<Node>, Box<Node>), // ElseIf(条件,ボディ)
    While(Box<Node>, Box<Node>),  // ElseIf(条件,ボディ)
    For(Box<Node>, Box<Node>),    // For(値、イテレータ|コレクション値)
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

    Increment(Box<Node>), // 増加(右辺)

    Decrement(Box<Node>), // 減少(右辺)

    AddAssign(Box<Node>, Box<Node>), // 加算代入(左辺,右辺)

    SubAssign(Box<Node>, Box<Node>), // 減算代入(左辺,右辺)

    MulAssign(Box<Node>, Box<Node>), // 乗算代入(左辺,右辺)

    DivAssign(Box<Node>, Box<Node>), // 除算代入(左辺,右辺)

    VariableDeclaration(Box<Node>, Box<Node>, Box<Node>), //変数定義(変数,型,右辺値)

    Assign(Box<Node>, Box<Node>), // 代入(変数,右辺値)
    Block(Vec<Box<Node>>),
    Variable(String),                                               // 変数(変数名)
    Number(i32),                                                    // 数値(i32の数値型)
    String(String),                                                 // 文字列(文字列)
    Bool(bool),                                                     // 真偽値(ブーリアン値)
    Unit(()),                                                       // Unit値(Void型)
    Function(String, Vec<String>, Box<Node>, Box<Node>, Box<Node>), // 関数定義(関数名,引数名リスト,ボディ,戻り値,戻り値の型)
    ReturnType(Box<Node>),                                          // 関数の戻り値の型(戻り値の型)
    DataType(Box<Node>),                                            // 変数の型
    Call(String, Vec<Node>),                                        // 関数呼び出し
    Return(Box<Node>),                                              // リターン
    MultiComment(Vec<String>, (usize, usize)),                      // 複数コメント
    SingleComment(String, (usize, usize)),                          // 単一コメント
    StatementEnd,                                                   // ステートメントの終わり
    Empty,
}
