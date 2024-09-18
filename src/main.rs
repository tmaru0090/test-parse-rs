mod decoder;
mod error;
mod lexer;
mod parser;
mod types;

use anyhow::{anyhow, Context, Result as R};
use decoder::*;
use env_logger;
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
    let file_name = if args.len() > 1 {
        &args[1]
    } else {
        "main.sc"
    };
    let mut decoder = match Decoder::load_script(file_name) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{}", e);
        Decoder::new()
        }
    };
    match decoder.decode() {
        Ok(v) => {
            info!("ret: {}", v);
            info!("ast_maps: {:?}", decoder.ast_map())
        }
        Err(e) => log::error!("{}", e),
    }
    Ok(())
}
