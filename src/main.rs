mod context;
mod decoder;
mod error;
mod lexer;
mod memory_mgr;
mod parser;
mod traits;
mod types;
use anyhow::{anyhow, Context, Result as R};
use decoder::*;
use env_logger;
use error::CompilerError;
use lexer::{Lexer, Token};
use log::info;
use parser::Node;
use parser::Parser;
use serde_json::to_string_pretty;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec::Vec;
use types::*;

fn remove_ansi_sequences(input: &str) -> String {
    input
        .replace("\u{1b}[31m", "")
        .replace("\u{1b}[0m", "")
        .replace("\u{1b}[38;2;100;100;200m", "")
        .replace("\u{1b}[0m", "")
}

fn main() -> R<(), String> {
    env_logger::init();
    let default_script_dir = std::path::Path::new("./script");
    std::env::set_current_dir(&default_script_dir)
        .expect("カレントディレクトリの設定に失敗しました");

    // コマンドライン引数を取得
    let args: Vec<String> = env::args().collect();
    let file_name = if args.len() > 1 { &args[1] } else { "main.sc" };
    let mut decoder = match Decoder::load_script(file_name) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            Decoder::new()
        }
    };

    match decoder.decode() {
        Ok(v) => {
            info!("ret: {}", v);
            //info!("ast_maps: {:?}", decoder.ast_map());
            info!("decode total-time: {:?}", decoder.decode_time())
        }
        Err(e) => eprintln!("{}", e),
    }
    Ok(())
}

/*
fn main() {
    let source_code = r#"
pub enum TokenType {
    /*基本算術演算子*/
Add, // +
Sub, // -
Mul, // *
Div, //
    #
}
"#;

    let formatted_errors = compile_group_error!(
        "warning",
        "src/types.rs",
        source_code,
        "multiple variants are never constructed",
        4, 8,
        5, 8,
        6, 8,
        7, 8
    );

    let formatted_errors_with_children = compile_error_with_children!(
        "error",
        "src/types.rs",
        source_code,
        8, 5,
        "不正なトークンを発見しました",
        "note",
        "`TokenType` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis"
    );

    println!("{}", formatted_errors);
    println!("{}", formatted_errors_with_children);
}
*/
/*
use serde_json::to_writer_pretty;

fn main()->R<(),String>{
    env_logger::init();
    let default_script_dir = std::path::Path::new("./script");
    std::env::set_current_dir(&default_script_dir)
        .expect("カレントディレクトリの設定に失敗しました");

    // コマンドライン引数を取得
    let args: Vec<String> = env::args().collect();
    let file_name = if args.len() > 1 { &args[1] } else { "main.sc" };
    
    let file = File::create("script-analysis/ast.json").map_err(|e|e.to_string())?;
    let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
    let tokens = Lexer::from_tokenize(file_name,content.clone())?;
    let nodes = Parser::from_parse(&tokens,file_name,content.clone())?;
    // 4. 構造体を整形済みのJSON形式にシリアライズしてファイルに書き込む
    to_writer_pretty(file, &nodes).map_err(|e|e.to_string());
    Ok(())
}
*/
