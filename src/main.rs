mod decoder;
mod parser;
mod tokenizer;
mod types;
use anyhow::{anyhow, Result as R};
use decoder::Decoder;
use env_logger;
use log::info;
use parser::Node;
use parser::Parser;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec::Vec;
use tokenizer::Tokenizer;
use types::*;

fn read_files_with_extension(extension: &str) -> R<Vec<String>> {
    let mut results = Vec::new();
    let current_dir = std::env::current_dir()?;

    // カレントディレクトリ内のすべてのファイルとディレクトリをリストアップ
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        // ファイルの拡張子が指定した拡張子と一致するか確認
        if path.is_file() && path.extension().map_or(false, |ext| ext == extension) {
            let file = fs::File::open(&path)?;
            let reader = io::BufReader::new(file);

            // ファイルの内容を読み取ってVec<String>に追加
            for line in reader.lines() {
                results.push(line?.trim().to_string());
            }
        }
    }

    if results.is_empty() {
        Err(anyhow!("No files with the extension {} found.", extension))
    } else {
        Ok(results)
    }
}

fn main() -> R<()> {
    env_logger::init();
    let mut test_src = String::new();
    let mut tokenizer = Tokenizer::new();
    let mut tokens = Vec::new();
    let extension = "script"; // 拡張子は "script" のみ

    test_src = String::from(
        r#"
        fn test(){ 
            let a = 100;
        }
    "#,
    );

    // .script ファイルが存在するか確認
    match read_files_with_extension(extension) {
        Ok(lines) => {
            //           info!("files: {:?}",lines.clone());
            tokenizer.set_input_vec(lines);
        }
        Err(_) => {
            // .script ファイルが存在しない場合はデフォルトのテストソースを使用
            tokenizer.set_input(test_src);
        }
    }

    tokens = tokenizer.tokenize()?;
    info!("tokens: ");
    info!("{:?}", tokens);
    let mut parser = Parser::new(&tokens);
    let nodes = parser.parse()?;

    info!("nodes: ");
    info!("{:?}", nodes);
    let mut decoder = Decoder::new();
    let block = Parser::<'_>::new_block(vec![
        *Parser::<'_>::new_add(
            Parser::<'_>::new_number(0),
            Parser::<'_>::new_number(0),
        )
    ]);
    decoder.register_function(
        "system_1".to_string(),
        vec![],
        block,
    ).unwrap();
    decoder.decode(&nodes).expect("Failed to decode");

    Ok(())
}
