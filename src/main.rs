mod decoder;
mod parser;
mod tokenizer;
mod types;
use anyhow::{anyhow, Result as R};
use decoder::*;
use env_logger;
use log::info;
use parser::Node;
use parser::Parser;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec::Vec;
use tokenizer::{Token, Tokenizer};
use types::*;
/*
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
                results.push(line?);
            }
        }
    }

    if results.is_empty() {
        Err(anyhow!("No files with the extension {} found.", extension))
    } else {
        Ok(results)
    }
}
    */

fn read_files_with_extension(extension: &str) -> R<Vec<String>> {
    let mut results = Vec::new();
    let current_dir = std::env::current_dir()?;

    // カレントディレクトリ内のすべてのファイルとディレクトリをリストアップ
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        // ファイルの拡張子が指定した拡張子と一致するか確認
        if path.is_file() && path.extension().map_or(false, |ext| ext == extension) {
            let file_contents = fs::read_to_string(&path)?;
            results.push(file_contents);
        }
    }

    if results.is_empty() {
        Err(anyhow!("No files with the extension {} found.", extension))
    } else {
        Ok(results)
    }
}
fn write_to_file(filename: &str, content: &str) -> R<()> {
    let mut file = File::create(filename)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
fn decode(nodes: &Vec<Node>) -> R<()> {
    #[cfg(feature = "decode")]
    {
        // my decode
        let mut decoder = Decoder::new();
        let block = Parser::<'_>::new_block(vec![*Parser::<'_>::new_return(
            Parser::<'_>::new_add(Parser::<'_>::new_number(100), Parser::<'_>::new_number(100)),
        )]);
        decoder
            .register_function("system_1".to_string(), vec![], block)
            .unwrap();

        decoder.decode(&nodes).expect("Failed to decode");
    }

    Ok(())
}
fn asm(nodes: &Vec<Node>) -> R<()> {
    // asm generate

    #[cfg(feature = "asm")]
    {
        let mut asm_i = AsmInterpreter::new();
        let block = Parser::<'_>::new_block(vec![*Parser::<'_>::new_return(
            Parser::<'_>::new_add(Parser::<'_>::new_number(100), Parser::<'_>::new_number(100)),
        )]);
        asm_i
            .register_function("system_1".to_string(), vec![], block)
            .unwrap();
        let asm_src = asm_i.generate_asm(&nodes).unwrap();
        write_to_file("main.asm", &asm_src)?;
    }

    Ok(())
}
fn main() -> R<()> {
    env_logger::init();
    let mut test_src = String::new();
    let mut tokenizer = Tokenizer::new();
    let mut tokens: Vec<Token> = Vec::new();
    let extension = "script"; // 拡張子は "script" のみ

    test_src = String::from("/* コメントでっせ\nにコメでっせ\n*/");

    // .script ファイルが存在するか確認
    match read_files_with_extension(extension) {
        Ok(lines) => {
            info!("files: {:?}", lines.clone());
            tokenizer.set_input_vec(lines);
        }
        Err(_) => {
            // .script ファイルが存在しない場合はデフォルトのテストソースを使用
            tokenizer.set_input(test_src);
        }
    }

    tokens = tokenizer.tokenize()?;
    let mut parser = Parser::new(&tokens);
    //let nodes = parser.parse()?;
    // デバッグ用
    // /*
    info!("tokens: ");
    info!("{:?}", tokens);
    //info!("nodes: ");
    //info!("{:?}", nodes);
    // */
    //asm(&nodes)?;
    //decode(&nodes)?;
    Ok(())
}
