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
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec::Vec;
use types::*;

fn read_files_with_extension(extension: &str) -> R<Vec<String>> {
    let mut results = Vec::new();
    let current_dir = std::env::current_dir()?;

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

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

fn read_files_with_path(path_str: &str) -> R<Vec<String>> {
    let mut results = Vec::new();
    let current_dir = std::env::current_dir()?;
    let target_path = current_dir.join(path_str);

    if target_path.is_file() {
        let file = fs::File::open(&target_path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            results.push(line?);
        }

        if results.is_empty() {
            Err(anyhow!("No content found in the file at path {}", path_str))
        } else {
            Ok(results)
        }
    } else {
        Err(anyhow!("No file found at path {}", path_str))
    }
}

fn write_to_file(filename: &str, content: &str) -> R<()> {
    let mut file = File::create(filename)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

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
    let file_name = "main.script";

    let mut decoder = Decoder::load_script(file_name)?;
    match decoder.decode() {
        Ok(v) => {
            info!("ret: {}", v);
            info!("ast_maps: {:?}", decoder.ast_map())
        }
        Err(e) => log::error!("{}", e),
    }
    Ok(())
}
