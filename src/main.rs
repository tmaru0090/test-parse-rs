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

fn decode(file_path: &str, content: String, nodes: &mut Vec<Box<Node>>) -> R<Value, String> {
    let mut value = Value::Null;
    #[cfg(feature = "decode")]
    {
        let mut decoder = Decoder::new(file_path.to_string(), content.clone());
        value = decoder.decode(nodes)?;
        return Ok(value);
    }
    Ok(value)
}

fn asm(nodes: &Vec<Box<Node>>, input: String, filename: &str) -> R<(), String> {
    #[cfg(feature = "asm")]
    {
        /*
        let mut asm_i = AsmInterpreter::new();
        asm_i.decode(&nodes)?;
        let asm_src = asm_i.get_asm_code();
        info!("{:?}", asm_src);
        write_to_file(filename, &asm_src).unwrap();
        */
    }
    Ok(())
}

fn main() -> R<(), String> {
    env_logger::init();
   let mut lexer = Lexer::new();
    let input_path = "./script/main.script";
    let mut input_content = String::new();
    match read_files_with_path(input_path) {
        Ok(lines) => {
           // 空行を取り除いて結合
            input_content = lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim())
                .collect::<Vec<&str>>()
                .join("\n");
            lexer.set_input_content_vec(vec![input_content.clone()]);
        }
        Err(_) => {}
    }

    //info!("input_content: {:?}", input_content.clone());
    let tokens = match lexer.tokenize() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
        }
    };
    let mut parser = Parser::new(&tokens, input_path.to_string(), input_content.clone());
    let mut nodes = match parser.parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
        }
    };

    match to_string_pretty(&tokens) {
        Ok(json) => info!("tokens: {}", json),
        Err(e) => info!("Failed to serialize tokens: {}", e),
    }

    match to_string_pretty(&nodes) {
        Ok(json) => {
            info!("nodes: {}", json);
            write_to_file("script-deps/ast.json", &json).unwrap();
        }
        Err(e) => info!("Failed to serialize nodes: {}", e),
    }

    let d = match decode(input_path, input_content.clone(), &mut nodes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            let err = remove_ansi_sequences(&e);
            write_to_file("script-deps/error.log", &err).unwrap();
            return Err(e);
        }
    };

    info!("{:?}", d);
    asm(&nodes, input_content.clone(), "main.asm").unwrap();
    Ok(())
}
