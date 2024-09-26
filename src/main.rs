mod context;
mod decoder;
mod error;
mod lexer;
mod memory_mgr;
mod parser;
mod traits;
mod types;

use anyhow::{anyhow, Context, Result as R};

#[cfg(any(feature = "full", feature = "decoder"))]
use decoder::interpreter::*;
use env_logger;
use error::CompilerError;
#[cfg(any(feature = "full", feature = "lexer"))]
use lexer::tokenizer::{Lexer, Token};
use log::info;

#[cfg(any(feature = "full", feature = "parser"))]
use parser::syntax::Node;
#[cfg(any(feature = "full", feature = "parser"))]
use parser::syntax::Parser;

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

fn main() -> R<(), String> {
    env_logger::init();
    let default_script_dir = std::path::Path::new("./script");
    std::env::set_current_dir(&default_script_dir)
        .expect("カレントディレクトリの設定に失敗しました");

    // コマンドライン引数を取得
    let args: Vec<String> = env::args().collect();
    let file_name = if args.len() > 1 { &args[1] } else { "main.sc" };

    /*デコード*/
    #[cfg(any(feature = "full", feature = "decoder"))]
    let mut decoder = match Decoder::load_script(file_name) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            Decoder::new()
        }
    };
    decoder
        .generate_doc(true)
        .generate_ast_file(true)
        .generate_error_log_file(true)
        .measured_decode_time(true);
    #[cfg(any(feature = "full", feature = "decoder"))]
    match decoder.decode() {
        Ok(v) => {
            info!("ret: {}", v);
            //info!("ast_maps: {:?}", decoder.ast_map());
            info!("decode total-time: {:?}", decoder.decode_time())
        }
        Err(e) => eprintln!("{}", e),
    }
    /*
    /*テスト用*/
    #[cfg(any(feature = "full", feature = "lexer"))]
    {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        info!("tokens: {:?}", tokens.clone());
        #[cfg(any(feature = "full", feature = "parser"))]
        {
            let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;

            info!("nodes: {:?}", nodes.clone());
        }
    }*/

    Ok(())
}
