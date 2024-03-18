use std::env;
use test_parse::decoder::*;
use test_parse::parser::*;
use test_parse::scope_manager::*;
use test_parse::tokenizer::*;

// ファイルから内容を取得
pub fn read_file(file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // ファイルを読み込む
    let contents = std::fs::read_to_string(file_name)?;
    // すべての式を評価して結果を表示

    Ok(contents)
}
//  メインエントリ
fn main() -> Result<(), String> {
    // 引数を処理
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("ファイル名を指定してください");
        return Err(format!("ファイル名がないお;;"));
    }
    let file_name = &args[1];
    match read_file(file_name) {
        Ok(contents) => {
            println!("ファイルの内容:\n{}", contents);
            let temp_src = String::from(contents);
            let src = temp_src.replace("\r", "");
            // トークナイズ
            let tokenizer = Tokenizer::new();
            let tokens = tokenizer.tokenize(&src)?;
            let mut parser = Parser::new(&tokens);
            println!("tokens: {:?}", tokens);
            let mut scope_manager = ScopeManager::new();
            // パース
            let nodes = parser.program()?;
            let mut decoder = Decoder::new(&parser, &mut scope_manager);
            // 実行
            decoder.decode(&nodes)?;
        }
        Err(e) => {
            eprintln!("ファイルを読み込めませんでした: {}", e);
        }
    }
    Ok(())
}
