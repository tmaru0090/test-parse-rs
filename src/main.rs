mod decoder;
mod error;
mod parser;
mod lexer;
mod types;
use anyhow::{anyhow, Context, Result as R};
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
use lexer::{Token, Lexer};
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
fn decode(nodes: &Vec<Node>, input: String) -> R<(), String> {
    #[cfg(feature = "decode")]
    {
        // my decode
        let mut decoder = Decoder::new(input.clone());
        decoder.decode(&nodes)?
    }

    Ok(())
}
fn asm(nodes: &Vec<Node>, input: String) -> R<(), String> {
    // asm generate

    #[cfg(feature = "asm")]
    {
        let mut asm_i = AsmInterpreter::new(input.clone());
        let asm_src = asm_i.generate_asm(&nodes);
        write_to_file("main.asm", &asm_src).unwrap();
    }
    Ok(())
}
fn main() -> R<(), String> {
    env_logger::init();
    let mut input_vec: Vec<String> = Vec::new();
    let mut test_src = String::new();
    let mut lexer = Lexer::new();
    let mut tokens: Vec<Token> = Vec::new();
    let extension = "script"; // 拡張子は "script" のみ

    test_src = String::from("/* コメントでっせ\nにコメでっせ\n*/");

    // .script ファイルが存在するか確認
    match read_files_with_extension(extension) {
        Ok(lines) => {
            info!("files: {:?}", lines.clone());
            lexer.set_input_vec(lines.clone());
            input_vec = lines.clone();
        }
        Err(_) => {
            // .script ファイルが存在しない場合はデフォルトのテストソースを使用
            lexer.set_input(test_src);
        }
    }
    let tokens = match lexer.tokenize() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
        }
    };

    let mut parser = Parser::new(&tokens, input_vec.join("\n"));
    let nodes = match parser.parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
        }
    };
    // デバッグ用
    info!("tokens: ");
    info!("{:?}", tokens);
    info!("nodes: ");
    info!("{:?}", nodes);
    //
    match decode(&nodes, input_vec.join("\n")) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
        }
    }
    asm(&nodes, input_vec.join("\n")).unwrap();
    Ok(())
}
