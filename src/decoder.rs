use crate::compile_error;
use crate::compile_group_error;
use crate::context::*;
use crate::error::CompilerError;
use crate::lexer::{Lexer, Token};
use crate::memory_mgr::*;
use crate::parser::Node;
use crate::parser::Parser;
use crate::traits::Size;
use crate::types::NodeValue;
use crate::types::*;
use anyhow::Result as R;
use chrono::{DateTime, Local, Utc};
use hostname::get;
use indexmap::IndexMap;
use log::info;
use property_rs::Property;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use serde_json::{Number, Value};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Add, Div, Mul, Sub};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use uuid::Uuid;
use whoami;
use win_msgbox::{
    AbortRetryIgnore, CancelTryAgainContinue, Icon, MessageBox, Okay, OkayCancel, RetryCancel,
    YesNo, YesNoCancel,
};

fn show_messagebox(
    message_type: &str,
    title: &str,
    message: &str,
    icon: Option<&str>,
) -> Option<String> {
    let msg_icon = match icon {
        Some("information") => Icon::Information,
        Some("warning") => Icon::Warning,
        Some("error") => Icon::Error,
        _ => return None, // デフォルトはアイコンなし
    };

    let response = match message_type {
        "okay" => {
            MessageBox::<Okay>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            Some("okay".to_string())
        }
        "yesno" => {
            let result = MessageBox::<YesNo>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                YesNo::Yes => Some("yes".to_string()),
                YesNo::No => Some("no".to_string()),
            }
        }
        "okaycancel" => {
            let result = MessageBox::<OkayCancel>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                OkayCancel::Okay => Some("okay".to_string()),
                OkayCancel::Cancel => Some("cancel".to_string()),
            }
        }
        "canceltryagaincontinue" => {
            let result = MessageBox::<CancelTryAgainContinue>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                CancelTryAgainContinue::Cancel => Some("cancel".to_string()),
                CancelTryAgainContinue::TryAgain => Some("tryagain".to_string()),
                CancelTryAgainContinue::Continue => Some("continue".to_string()),
            }
        }
        "retrycancel" => {
            let result = MessageBox::<RetryCancel>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                RetryCancel::Retry => Some("retry".to_string()),
                RetryCancel::Cancel => Some("cancel".to_string()),
            }
        }

        _ => {
            MessageBox::<Okay>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            Some("okay".to_string())
        }
    };

    response
}

// メイン実行環境
#[derive(Debug, Clone, Property)]
pub struct Decoder {
    #[property(get)]
    ast_map: IndexMap<String, Vec<Box<Node>>>, // ASTのリスト(ファイル名,Nodeのベクター)
    #[property(get)]
    memory_mgr: MemoryManager, // メモリーマネージャー
    #[property(get)]
    context: Context, // コンテキスト
    #[property(get)]
    file_contents: IndexMap<String, String>, // ファイルの内容(ファイル名,ファイルの内容)
    #[property(get)]
    current_node: Option<(String, Box<Node>)>, // 現在のノード(ファイル名,現在のNode)

    #[property(get)]
    generated_ast_file: bool, // ASTの生成をするかどうか

    #[property(get)]
    generated_error_log_file: bool, // エラーログを生成するかどうか

    #[property(get)]
    measure_decode_time: bool, // 実行時間を計測するかどうか

    #[property(get)]
    decode_time: f32, // 実行時間

    #[property(get)]
    entry_func: (bool, String), // main関数の有無(フラグ,見つかった関数名(main|Main))
}
impl Decoder {
    pub fn measured_decode_time(&mut self, flag: bool) -> &mut Self {
        self.measure_decode_time = flag;
        self
    }
    pub fn generate_ast_file(&mut self, flag: bool) -> &mut Self {
        self.generated_ast_file = flag;
        self
    }
    pub fn generate_error_log_file(&mut self, flag: bool) -> &mut Self {
        self.generated_error_log_file = flag;
        self
    }
    // 現在のASTのマップの先頭に指定スクリプトのASTを追加
    pub fn add_first_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        // 最初に要素を挿入するために新しい IndexMap を作る
        let mut new_ast_map = IndexMap::new();
        new_ast_map.insert(file_name.to_string(), nodes.clone());

        // 既存の ast_map の要素を新しいものに追加
        new_ast_map.extend(self.ast_map.drain(..));

        // ast_map を新しいものに置き換える
        self.ast_map = new_ast_map;

        Ok(self)
    }

    // 現在のASTのマップに指定スクリプトのASTを追加
    pub fn add_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        self.ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(self)
    }

    // 現在のASTのマップに文字列でスクリプトのASTを追加
    pub fn add_ast_from_text(&mut self, file_name: &str, content: &str) -> R<&mut Self, String> {
        // トークン化処理
        let tokens = Lexer::from_tokenize(file_name, content.to_string())?;

        // パース処理
        let nodes = Parser::from_parse(&tokens, file_name, content.to_string())?;

        // ASTをマップに追加
        self.ast_map.insert(file_name.to_string(), nodes.clone());

        // 成功時にselfを返す
        Ok(self)
    }
    // スクリプトを読み込む
    pub fn load_script(file_name: &str) -> R<Self, String> {
        let mut ast_map: IndexMap<String, Vec<Box<Node>>> = IndexMap::new();
        let file_content = std::fs::read_to_string(file_name)
            .map_err(|e| e.to_string())
            .expect("Failed to script file");

        let tokens = Lexer::from_tokenize(file_name, file_content.clone())?;

        info!("tokens: {:?}", tokens.clone());

        let nodes = Parser::from_parse(&tokens, file_name, file_content.clone())?;

        info!("nodes: {:?}", nodes.clone());
        ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(Decoder {
            ast_map,
            memory_mgr: MemoryManager::new(1024 * 1024),
            file_contents: IndexMap::new(),
            current_node: None,
            context: Context::new(),
            generated_ast_file: true,
            generated_error_log_file: true,
            measure_decode_time: true,
            decode_time: 0.0,
            entry_func: (false, String::new()),
        })
    }
    pub fn new() -> Self {
        Self {
            ast_map: IndexMap::new(),
            memory_mgr: MemoryManager::new(1024 * 1024),
            file_contents: IndexMap::new(),
            current_node: None,
            context: Context::new(),
            generated_ast_file: true,
            generated_error_log_file: true,
            measure_decode_time: true,
            decode_time: 0.0,
            entry_func: (false, String::new()),
        }
    }
    fn get_value_size(&self, v_type: &str, v_value: &Value) -> usize {
        match v_type {
            "void" | "unit" => 0,
            "i32" => std::mem::size_of::<i32>(),
            "i64" => std::mem::size_of::<i64>(),
            "f32" => std::mem::size_of::<f32>(),
            "f64" => std::mem::size_of::<f64>(),
            "string" => {
                if let Value::String(ref s) = v_value {
                    s.len()
                } else {
                    0
                }
            }
            _ => serde_json::to_vec(v_value).unwrap().len(),
        }
    }
    fn infer_type(&self, value: &Value) -> String {
        match value {
            Value::Array(_) => "array".to_string(),
            Value::Null => "void".to_string(),
            Value::Number(num) => {
                if num.is_i64() {
                    let i_value = num.as_i64().unwrap();
                    if i_value >= i32::MIN as i64 && i_value <= i32::MAX as i64 {
                        "i32".to_string()
                    } else {
                        "i64".to_string()
                    }
                } else if num.is_f64() {
                    let f_value = num.as_f64().unwrap();
                    if f_value >= f32::MIN as f64 && f_value <= f32::MAX as f64 {
                        "f32".to_string()
                    } else {
                        "f64".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            }
            Value::String(_) => "string".to_string(),
            Value::Bool(_) => "bool".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn check_type(&self, value: &Value, expected_type: &str) -> R<Value, String> {
        let (file_name, node) = if let Some((file_name, node)) = self.current_node.clone() {
            (file_name, node)
        } else {
            (String::new(), Box::new(Node::default()))
        };
        // 型定義が存在するか確認
        if !self.context.type_context.contains_key(expected_type) {
            return Err(compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &file_name,
                &self.file_contents.get(&file_name).unwrap(),
                "Type '{}' is not defined",
                expected_type
            ));
        }

        match expected_type {
            "unit" | "void" => Ok(Value::Null),
            "i32" => {
                if let Some(num) = value.as_i64() {
                    match i32::try_from(num) {
                        Ok(num_i32) => Ok(Value::Number(serde_json::Number::from(num_i32))),
                        Err(_) => Err(compile_error!(
                            "error",
                            node.clone().line(),
                            node.clone().column(),
                            &file_name,
                            &self.file_contents.get(&file_name).unwrap(),
                            "Value out of range for i32: {:?}",
                            num
                        )),
                    }
                } else {
                    Err(compile_error!(
                        "error",
                        node.clone().line(),
                        node.clone().column(),
                        &file_name,
                        &self.file_contents.get(&file_name).unwrap(),
                        "Type mismatch for i32: {:?}",
                        value
                    ))
                }
            }
            "i64" => {
                if let Some(num) = value.as_i64() {
                    Ok(Value::Number(serde_json::Number::from(num)))
                } else {
                    Err(compile_error!(
                        "error",
                        node.clone().line(),
                        node.clone().column(),
                        &file_name,
                        &self.file_contents.get(&file_name).unwrap(),
                        "Type mismatch for i64: {:?}",
                        value
                    ))
                }
            }
            "f32" => {
                if let Some(num) = value.as_f64() {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(num as f64)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Err(compile_error!(
                        "error",
                        node.clone().line(),
                        node.clone().column(),
                        &file_name,
                        &self.file_contents.get(&file_name).unwrap(),
                        "Type mismatch for f32: {:?}",
                        value
                    ))
                }
            }
            "f64" => {
                if let Some(num) = value.as_f64() {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(num)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Err(compile_error!(
                        "error",
                        node.clone().line(),
                        node.clone().column(),
                        &file_name,
                        &self.file_contents.get(&file_name).unwrap(),
                        "Type mismatch for f64: {:?}",
                        value
                    ))
                }
            }
            _ => Ok(value.clone()),
        }
    }

    pub fn decode(&mut self) -> R<Value, String> {
        // 実行にかかった時間を計測
        let start_time = if self.measure_decode_time {
            Some(Instant::now())
        } else {
            None
        };
        let mut value = Value::Null;
        let original_node = self.current_node.clone();

        // ASTを評価して実行
        let mut evaluated_files = std::collections::HashSet::new();

        for (file_name, nodes) in self.ast_map() {
            if evaluated_files.contains(&file_name) {
                continue; // 既に評価済みのファイルはスキップ
            }
            evaluated_files.insert(file_name.clone());

            self.current_node = Some((file_name.clone(), Box::new(Node::default())));
            let content = std::fs::read_to_string(file_name.clone()).map_err(|e| e.to_string())?;
            self.file_contents.insert(file_name.clone(), content);
            for node in nodes {
                self.current_node = Some((file_name.clone(), node.clone()));
                value = self.execute_node(&node)?;
            }
        }
        // メインエントリーが定義されていたら実行
        if self.entry_func.0 {
            self.add_ast_from_text("main-entry", &format!("{}();", self.entry_func.1))?;
            if let Some((key, value_node)) = self.ast_map.clone().iter().last() {
                for node in value_node {
                    value = self.execute_node(node)?;
                }
            }
        }

        self.current_node = original_node;
        if self.generated_ast_file {
            // ディレクトリが存在しない場合は作成
            std::fs::create_dir_all("./script-analysis").map_err(|e| e.to_string())?;
            // IndexMapをHashMapに変換
            let ast_map: std::collections::HashMap<_, _> =
                self.ast_map.clone().into_iter().collect();
            let ast_json = to_string_pretty(&ast_map).map_err(|e| e.to_string())?;
            std::fs::write("./script-analysis/ast.json", ast_json).map_err(|e| e.to_string())?;
        }
        if let Some(start) = start_time {
            let duration = start.elapsed();
            // 秒とナノ秒を取得
            let secs = duration.as_secs() as f32;
            let nanos = duration.subsec_nanos() as f32;
            self.decode_time = secs + (nanos / 1_000_000_000.0);
        }
        Ok(value)
    }

    fn eval_block(&mut self, block: &Vec<Box<Node>>) -> R<Value, String> {
        let mut result = Value::Null;
        let initial_local_context = self.context.local_context.clone(); // 現在のローカルコンテキストを保存
        for b in block {
            info!("local_context: {:?}", self.context.local_context.clone());
            result = self.execute_node(b)?;
        }
        self.context.local_context = initial_local_context; // ブロックの処理が終わったらローカルコンテキストを元に戻す
        Ok(result)
    }

    fn eval_include(&mut self, file_name: &String) -> R<Value, String> {
        self.add_first_ast_from_file(file_name)?;
        let ast_map = self.ast_map();
        let nodes = ast_map.get(file_name).unwrap();
        let mut result = Value::Null;
        for node in nodes {
            //info!("{:?}",node.clone());
            result = self.execute_node(&node)?;
        }
        Ok(result)
    }

    fn eval_single_comment(
        &mut self,
        content: &String,
        lines: &(usize, usize),
    ) -> R<Value, String> {
        self.context
            .comment_lists
            .insert((lines.0, lines.1), vec![content.clone()]);
        info!("MultiComment added at line {}, column {}", lines.0, lines.1);
        Ok(Value::Null)
    }
    fn eval_multi_comment(
        &mut self,
        content: &Vec<String>,
        lines: &(usize, usize),
    ) -> R<Value, String> {
        self.context
            .comment_lists
            .insert((lines.0, lines.1), content.clone().to_vec());
        info!("MultiComment added at line {}, column {}", lines.0, lines.1);
        Ok(Value::Null)
    }

    fn eval_array(&mut self, data_type: &Box<Node>, values: &Vec<Box<Node>>) -> R<Value, String> {
        // 型を評価
        let v_type = match data_type.node_value() {
            NodeValue::DataType(d) => self.execute_node(&d)?,
            _ => Value::Null,
        };

        // 各値を評価し、型チェックを行う
        let mut array = Vec::new();
        for value in values {
            let v_value = self.execute_node(&*value)?;
            //self.check_type(&v_value, v_type.as_str().unwrap_or(""))?;
            array.push(v_value);
        }

        // 配列全体をヒープにコピー
        self.memory_mgr.allocate(array.clone());
        // 結果を返す
        Ok(Value::Array(array.clone()))
    }

    fn eval_assign(
        &mut self,
        node: &Node,
        var_name: &Box<Node>,
        value: &Box<Node>,
        index: &Box<Node>,
    ) -> R<Value, String> {
        let mut result = Value::Null;

        // ステートメントフラグのチェック
        if !node.is_statement() {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable Assign must be a statement"
            ));
        }

        let name = match var_name.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&name)
            .cloned()
            .or_else(|| self.context.global_context.get(&name).cloned());

        if let Some(mut variable) = variable_data {
            if variable.is_mutable {
                //panic!("{:?}",variable);

                //panic!("{:?}",value);
                let new_value = self.execute_node(&value)?;
                self.check_type(&new_value, variable.data_type.as_str().unwrap_or(""))?;

                match &mut variable.value {
                    Value::Array(ref mut array) => {
                        let index_value = self.execute_node(&index)?;
                        if let Value::Number(n) = index_value {
                            let index_usize = n.as_u64().unwrap_or(0) as usize;
                            if index_usize < array.len() {
                                array[index_usize] = new_value.clone();
                                result = new_value.clone();
                            } else {
                                return Err(compile_error!(
                                    "error",
                                    self.current_node.clone().unwrap().1.line(),
                                    self.current_node.clone().unwrap().1.column(),
                                    &self.current_node.clone().unwrap().0,
                                    &self
                                        .file_contents
                                        .get(&self.current_node.clone().unwrap().0)
                                        .unwrap(),
                                    "Index out of bounds"
                                ));
                            }
                        } else {
                            return Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Index is not a number"
                            ));
                        }
                    }
                    _ => {
                        variable.value = new_value.clone();
                        result = new_value.clone();
                    }
                }

                self.memory_mgr
                    .update_value(variable.address.clone(), variable.value.clone());

                if self.context.local_context.contains_key(&name) {
                    self.context.local_context.insert(name.clone(), variable);
                } else {
                    self.context.global_context.insert(name.clone(), variable);
                }

                info!("Assign: name = {:?}, new_value = {:?}", name, result);
                Ok(result)
            } else {
                Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line(),
                    self.current_node.clone().unwrap().1.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Variable '{}' is not mutable",
                    name
                ))
            }
        } else {
            Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable '{}' is not defined",
                name
            ))
        }
    }

    fn eval_call(&mut self, name: &String, args: &Vec<Node>, is_system: &bool) -> R<Value, String> {
        let mut result = Value::Null;
        let mut evaluated_args = Vec::new();
        for arg in args {
            let evaluated_arg = self.execute_node(&arg)?;
            info!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        {
            if *is_system {
                match name.as_str() {
                    "str" => {
                        if args.len() != 1 {
                            return Err("to_str expects exactly one argument".into());
                        }
                        let n = match self.execute_node(&args[0])? {
                            Value::Number(v) => v,
                            _ => return Err("to_str expects a string as the file name".into()),
                        };
                        let string = n.to_string();
                        return Ok(serde_json::json!(string));
                    }
                    "show_msg_box" => {
                        if args.len() != 4 {
                            return Err("show_msg_box expects exactly two arguments".into());
                        }
                        let message_type = match self.execute_node(&args[0])? {
                            Value::String(v) => v,
                            _ => {
                                return Err("show_msg_box expects a string as the file name".into())
                            }
                        };
                        let title = match self.execute_node(&args[1])? {
                            Value::String(v) => v,
                            _ => {
                                return Err("show_msg_box expects a string as the file name".into())
                            }
                        };
                        let message = match self.execute_node(&args[2])? {
                            Value::String(v) => v,
                            _ => {
                                return Err("show_msg_box expects a string as the file name".into())
                            }
                        };
                        let icon = match self.execute_node(&args[3])? {
                            Value::String(v) => Some(v),
                            _ => None,
                        };
                        let responce =
                            show_messagebox(&message_type, &title, &message, icon.as_deref());
                        return Ok(serde_json::json!(responce));
                    }
                    "write_at_file" => {
                        if args.len() != 3 {
                            return Err("write_file expects exactly two arguments".into());
                        }
                        let file_name = match self.execute_node(&args[0])? {
                            Value::String(v) => v,
                            _ => return Err("write_file expects a string as the file name".into()),
                        };
                        let insert_str = match self.execute_node(&args[1])? {
                            Value::String(v) => v,
                            Value::Array(arr) => arr
                                .into_iter()
                                .map(|v| {
                                    if let Value::String(s) = v {
                                        Ok::<String, String>(s)
                                    } else {
                                        Err("write_file expects an array of strings as the content"
                                            .into())
                                    }
                                })
                                .collect::<Result<Vec<String>, String>>()?
                                .join("\n"),
                            _ => return Err(
                                "write_file expects a string or an array of strings as the content"
                                    .into(),
                            ),
                        };
                        let pos = match self.execute_node(&args[2])? {
                            Value::Number(v) => v.as_u64().unwrap(),
                            _ => 0,
                        };
                        // ファイルを開く
                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open(&file_name)
                            .unwrap();

                        // 既存の内容をすべて読み込む
                        let mut content = String::new();
                        file.read_to_string(&mut content).unwrap();
                        /*
                                                // 挿入する位置までのバイト数を計算
                                                let split_index = pos as usize;

                                                // 挿入位置に文字列を挿入
                                                let (head, tail) = content.split_at(split_index);
                        */

                        // posバイト目ではなく、pos文字目で分割する
                        let char_pos = content
                            .char_indices()
                            .nth(pos as usize)
                            .map(|(i, _)| i)
                            .unwrap_or(content.len());

                        // 挿入位置に文字列を挿入
                        let (head, tail) = content.split_at(char_pos);

                        let new_content = format!("{}{}{}", head, insert_str, tail);

                        // ファイルの内容を書き換えるためにシークしてから書き込み
                        file.seek(SeekFrom::Start(0)).unwrap();
                        file.write_all(new_content.as_bytes()).unwrap();

                        return Ok(Value::Null);
                    }
                    "open_recent" => {
                        if !args.is_empty() {
                            return Err("open_recent expects no arguments".into());
                        }

                        // 最近使用したアイテムフォルダのパス
                        let recent_folder =
                            std::env::var("APPDATA").unwrap() + r"\Microsoft\Windows\Recent";

                        // フォルダ内のファイルを取得
                        let paths = std::fs::read_dir(recent_folder)
                            .unwrap()
                            .filter_map(Result::ok)
                            .map(|entry| entry.path())
                            .collect::<Vec<std::path::PathBuf>>();
                        let recent_lists = serde_json::json!(paths);

                        return Ok(recent_lists.clone());
                    }

                    "sleep" => {
                        if args.len() != 1 {
                            return Err("sleep expects exactly one argument".into());
                        }
                        let duration = match self.execute_node(&args[0])? {
                            Value::Number(v) => v,
                            _ => return Err("read_file expects a string as the file name".into()),
                        };
                        sleep(std::time::Duration::from_secs(duration.as_u64().unwrap()));
                        return Ok(Value::Null);
                    }

                    "read_file" => {
                        if args.len() != 1 {
                            return Err("read_file expects exactly one argument".into());
                        }
                        let file_name = match self.execute_node(&args[0])? {
                            Value::String(v) => v,
                            _ => return Err("read_file expects a string as the file name".into()),
                        };
                        let mut file = File::open(file_name).unwrap();
                        let mut contents = String::new();
                        file.read_to_string(&mut contents).unwrap();
                        return Ok(Value::String(contents));
                    }

                    "write_file" => {
                        if args.len() != 2 {
                            return Err("write_file expects exactly two arguments".into());
                        }
                        let file_name = match self.execute_node(&args[0])? {
                            Value::String(v) => v,
                            _ => return Err("write_file expects a string as the file name".into()),
                        };
                        let content = match self.execute_node(&args[1])? {
                            Value::String(v) => v,
                            Value::Array(arr) => arr
                                .into_iter()
                                .map(|v| {
                                    if let Value::String(s) = v {
                                        Ok::<String, String>(s)
                                    } else {
                                        Err("write_file expects an array of strings as the content"
                                            .into())
                                    }
                                })
                                .collect::<Result<Vec<String>, String>>()?
                                .join("\n"),
                            _ => return Err(
                                "write_file expects a string or an array of strings as the content"
                                    .into(),
                            ),
                        };
                        let mut file = File::create(file_name).unwrap();
                        file.write_all(content.as_bytes()).unwrap();

                        return Ok(Value::Null);
                    }

                    "print" => {
                        let format = match self.execute_node(&args[0])? {
                            Value::String(v) => v,
                            _ => return Err("print expects a string as the format".into()),
                        };
                        let args = self.execute_node(&args[1])?;

                        match args {
                            Value::Array(arr) => {
                                let mut formatted_args: Vec<String> = arr
                                    .iter()
                                    .map(|arg| match arg {
                                        Value::String(s) => s.clone(),
                                        Value::Number(n) => n.to_string(),
                                        Value::Bool(b) => b.to_string(),
                                        _ => format!("{:?}", arg),
                                    })
                                    .collect();

                                let mut formatted_string = format.clone();
                                for arg in arr {
                                    if formatted_string.contains("{:?}") {
                                        formatted_string = formatted_string.replacen(
                                            "{:?}",
                                            &format!("{:?}", arg),
                                            1,
                                        );
                                    } else {
                                        formatted_string = formatted_string.replacen(
                                            "{}",
                                            &formatted_args.remove(0),
                                            1,
                                        );
                                    }
                                }
                                println!("{}", formatted_string);
                            }
                            _ => return Err("print expects an array of arguments".into()),
                        }
                        return Ok(Value::Null);
                    }

                    "println" => {
                        for arg in args {
                            let value = self.execute_node(arg)?;
                            print!("{}", value);
                        }
                        println!();
                        return Ok(Value::Null);
                    }
                    "exit" => {
                        if args.len() != 1 {
                            return Err("exit expects exactly one argument".into());
                        }
                        let status = match self.execute_node(&args[0])? {
                            Value::Number(n) => {
                                n.as_i64().ok_or("exit expects a positive integer")?
                            }
                            _ => return Err("exit expects a number as the status".into()),
                        };
                        std::process::exit(status.try_into().unwrap());

                        return Ok(Value::Null);
                    }
                    "args" => {
                        if !args.is_empty() {
                            return Err("args expects no arguments".into());
                        }
                        let args: Vec<String> = std::env::args().collect();
                        let value: Value =
                            Value::Array(args.into_iter().map(Value::String).collect());
                        return Ok(value);
                    }
                    "cmd" => {
                        if evaluated_args.len() < 1 {
                            return Err("cmd expects at least one argument".into());
                        }
                        let command = match &evaluated_args[0] {
                            Value::String(v) => v.clone(),
                            _ => return Err("cmd expects the first argument to be a string".into()),
                        };
                        let command_args =
                            if evaluated_args.len() > 1 {
                                match &evaluated_args[1] {
                                    Value::Array(v) => v
                                        .iter()
                                        .filter_map(|item| {
                                            if let Value::String(s) = item {
                                                Some(s.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect(),
                                    _ => return Err(
                                        "cmd expects the second argument to be an array of strings"
                                            .into(),
                                    ),
                                }
                            } else {
                                Vec::new()
                            };
                        let output = Command::new(command)
                            .args(&command_args)
                            .output()
                            .expect("外部コマンドの実行に失敗しました");
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        return Ok(Value::Array(vec![
                            Value::String(stdout),
                            Value::String(stderr),
                        ]));
                    }
                    // 他のシステム関数の処理...
                    _ => return Err(format!("Unknown function: {}", name)),
                }
            }
        }
        let func_name = name;
        let variables = {
            let global_context = &self.context.global_context;
            global_context
                .get(func_name.as_str())
                .cloned()
                .ok_or_else(|| {
                    compile_error!(
                        "error",
                        self.current_node.clone().unwrap().1.line(),
                        self.current_node.clone().unwrap().1.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Function '{}' is not defined",
                        func_name
                    )
                })?
        };

        let func_address = variables.address;
        let func_info = self.memory_mgr.get_value::<Value>(func_address).unwrap();
        let _args = func_info["args"].clone();
        let _body = func_info["body"].clone();
        let body: Node = serde_json::from_value(_body).unwrap();
        let return_type = func_info["return_type"].clone();

        // スタックフレームをプッシュ
        self.memory_mgr.push_stack_frame(func_name);

        for (arg, value) in _args.as_array().unwrap().iter().zip(&evaluated_args) {
            let arg_name = arg["name"].as_str().unwrap();
            let arg_type = arg["type"].clone();
            let index = self.memory_mgr.allocate(value.clone());
            let block = MemoryBlock {
                id: index,
                value: Box::new(value.clone()),
            };
            self.memory_mgr.add_to_stack_frame(func_name, block);
            self.context.local_context.insert(
                arg_name.to_string(),
                Variable {
                    value: value.clone(),
                    data_type: arg_type.clone(),
                    address: index,
                    is_mutable: false,
                    size: 0,
                },
            );
        }

        let _body = match body.clone().node_value() {
            NodeValue::Block(v) => v,
            _ => vec![],
        };
        let b = _body
            .iter()
            .filter(|node| node.node_value() != NodeValue::Unknown)
            .collect::<Vec<_>>();
        for body in b {
            result = self.execute_node(&body)?;
        }

        // スタックフレームをポップ
        self.memory_mgr.pop_stack_frame(func_name);

        info!(
            "CallFunction: name = {:?},args = {:?},return_value = {:?}",
            func_name,
            evaluated_args.clone(),
            result
        );

        Ok(result)
    }
    fn eval_function(
        &mut self,
        name: &String,
        args: &Vec<(Box<Node>, String)>,
        body: &Box<Node>,
        return_type: &Box<Node>,
        is_system: &bool,
    ) -> R<Value, String> {
        let func_name = name; // すでに String 型なのでそのまま使う
        if func_name == "main" || func_name == "Main" {
            self.entry_func.0 = true;
            self.entry_func.1 = func_name.clone();
        }
        self.check_reserved_words(&func_name, RESERVED_WORDS)?;

        // 関数がすでに定義されているかチェック
        if self.context.global_context.contains_key(func_name.as_str()) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Function '{}' is already defined",
                func_name
            ));
        }

        let mut arg_addresses = Vec::new();

        let func_index = self.memory_mgr.allocate(func_name.clone());

        for (i, (data_type, arg_name)) in args.iter().enumerate() {
            arg_addresses.push(serde_json::json!({"name": arg_name.clone(),"type": data_type}));
        }

        // 関数の情報をシリアライズしてヒープに格納
        let func_info = serde_json::json!({
            "args": arg_addresses,
            "body": body,
            "return_type": return_type,
        });
        let func_info_index = self.memory_mgr.allocate(func_info.clone());

        if *is_system {
            // 関数の情報をグローバルコンテキストに保存
            self.context.global_context.insert(
                format!("@{}", func_name.clone()),
                Variable {
                    value: func_info.clone(),
                    data_type: Value::String("Function".into()),
                    address: func_info_index,
                    is_mutable: false,
                    size: 0,
                },
            );
        }
        // 関数の情報をグローバルコンテキストに保存
        self.context.global_context.insert(
            func_name.clone(),
            Variable {
                value: func_info.clone(),
                data_type: Value::String("Function".into()),
                address: func_info_index,
                is_mutable: false,
                size: 0,
            },
        );

        info!(
            "FunctionDeclaration: name = {:?}, args = {:?}, body = {:?}, return_type = {:?}",
            func_name, arg_addresses, body, return_type
        );
        Ok(Value::Null)
    }

    fn check_reserved_words(&self, input: &str, reserved_words: &[&str]) -> Result<Value, String> {
        if reserved_words.contains(&input) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "'{}' is a reserved word",
                input
            ));
        } else {
            Ok(Value::Null)
        }
    }
    fn eval_variable_declaration(
        &mut self,
        node: &Node,
        var_name: &Box<Node>,
        data_type: &Box<Node>,
        value: &Box<Node>,
        is_local: &bool,
        is_mutable: &bool,
        is_reference: &bool,
    ) -> R<Value, String> {
        // ステートメントフラグのチェック
        if !node.is_statement() {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable declaration must be a statement"
            ));
        }

        //info!("is_reference: {:?}", is_reference);
        let name = match var_name.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

        self.check_reserved_words(&name, RESERVED_WORDS)?;

        let mut v_type = Value::Null;
        let v_value;
        let address;

        {
            // 一時的にcontextの借用を解除
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            if context.contains_key(&name) {
                return Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line(),
                    self.current_node.clone().unwrap().1.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Variable '{}' is already defined",
                    name
                ));
            }

            let v = match data_type.node_value() {
                NodeValue::DataType(v_type) => match v_type.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                },
                _ => String::new(),
            };
            v_value = {
                let _value = self.execute_node(&value)?;
                //            self.check_type(&_value, v_type.as_str().unwrap_or(""))?
                _value.clone()
            };
            v_type = Value::String(v.into());
        }

        if *is_reference {
            // 参照型の場合、右辺の変数名を取り出してアドレスを取得して直接変更
            address = {
                let context = if *is_local {
                    &mut self.context.local_context
                } else {
                    &mut self.context.global_context
                };

                match value.node_value() {
                    NodeValue::Variable(v) => {
                        if let Some(variable) = context.get(&v) {
                            variable.address
                        } else {
                            return Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' not found in context",
                                v
                            ));
                        }
                    }
                    _ => {
                        let _address = self.memory_mgr.allocate(v_value.clone());
                        _address
                    }
                }
            };

            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_type: v_type.clone(),
                    address,
                    is_mutable: *is_mutable,
                    size: v_value.size(),
                },
            );
        } else {
            address = self.memory_mgr.allocate(v_value.clone());
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_type: v_type.clone(),
                    address,
                    is_mutable: *is_mutable,
                    size: v_value.size(),
                },
            );
        }

        info!("VariableDeclaration: name = {:?}, data_type = {:?}, value = {:?}, address = {:?} is_mutable: {} is_local: {}", name, v_type, v_value, address,is_mutable,is_local);
        let line = self.current_node.clone().unwrap().1.line();
        let column = self.current_node.clone().unwrap().1.column();
        self.context
            .used_context
            .insert(name.clone(), (line, column, false));
        Ok(v_value)
    }

    fn eval_type_declaration(
        &mut self,
        _type_name: &Box<Node>,
        _type: &Box<Node>,
    ) -> R<Value, String> {
        let name = match _type_name.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };
        if self.context.type_context.contains_key(&name) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "type '{}' is already defined",
                name
            ));
        }
        let v_type = match _type.node_value() {
            NodeValue::String(v) => v,
            _ => String::new(),
        };

        // 型定義をtype_contextに保存
        self.context
            .type_context
            .insert(name.clone(), v_type.clone());

        info!(
            "TypeDeclaration: type_name = {:?}, type = {:?}",
            name, v_type
        );
        Ok(Value::String(name.into()))
    }
    fn eval_variable(&mut self, name: &String) -> R<Value, String> {
        let line = self.current_node.clone().unwrap().1.line();
        let column = self.current_node.clone().unwrap().1.column();
        self.context
            .used_context
            .insert(name.clone(), (line, column, true));

        if let Some(var) = self.context.local_context.get(name) {
            // ローカルスコープで変数を見つけた場合
            let index = var.address; // アドレスを取得
            let value_size = self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

            info!(
                "Found variable: Name = {} Address = {}, Value size = {}, Heap size = {}",
                name,
                index,
                value_size,
                self.memory_mgr.heap.len()
            );

            let value = self
                .memory_mgr
                .get_value::<Value>(index)
                .expect("Failed to retrieve value");

            Ok(value.clone())
        } else if let Some(var) = self.context.global_context.get(name) {
            // グローバルスコープで変数を見つけた場合
            let index = var.address; // アドレスを取得
            let value_size = self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

            info!(
                "Found variable: Name = {} Address = {}, Value size = {}, Heap size = {}",
                name,
                index,
                value_size,
                self.memory_mgr.heap.len()
            );

            let value = self
                .memory_mgr
                .get_value::<Value>(index)
                .expect("Failed to retrieve value");
            Ok(value.clone())
        } else {
            Ok(Value::Null)
        }
    }
    fn eval_return(&mut self, ret: &Box<Node>) -> R<Value, String> {
        let ret = self.execute_node(&ret)?;
        info!("Return: {:?}", ret);
        Ok(ret)
    }

    fn eval_callback_function(
        &mut self,
        name: &String,
        args: &Vec<(Box<Node>, String)>,
        body: &Box<Node>,
        return_type: &Box<Node>,
        is_system: &bool,
    ) -> R<Value, String> {
        let func_name = name; // すでに String 型なのでそのまま使う
        if func_name == "main" || func_name == "Main" {
            self.entry_func.0 = true;
            self.entry_func.1 = func_name.clone();
        }
        // 関数がすでに定義されているかチェック
        if self.context.global_context.contains_key(func_name.as_str()) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Function '{}' is already defined",
                func_name
            ));
        }

        let mut arg_addresses = Vec::new();

        let func_index = self.memory_mgr.allocate(func_name.clone());

        for (i, (data_type, arg_name)) in args.iter().enumerate() {
            arg_addresses.push(serde_json::json!({"name": arg_name.clone(),"type": data_type}));
        }

        // 関数の情報をシリアライズしてヒープに格納
        let func_info = serde_json::json!({
            "args": arg_addresses,
            "body": body,
            "return_type": return_type,
        });
        let func_info_index = self.memory_mgr.allocate(func_info.clone());

        if *is_system {
            // 関数の情報をグローバルコンテキストに保存
            self.context.global_context.insert(
                format!("@{}", func_name.clone()),
                Variable {
                    value: func_info.clone(),
                    data_type: Value::String("Function".into()),
                    address: func_info_index,
                    is_mutable: false,
                    size: 0,
                },
            );
        }
        // 関数の情報をグローバルコンテキストに保存
        self.context.global_context.insert(
            func_name.clone(),
            Variable {
                value: func_info.clone(),
                data_type: Value::String("CallBackFunction".into()),
                address: func_info_index,
                is_mutable: false,
                size: 0,
            },
        );

        info!(
            "CallBack FunctionDeclaration: name = {:?}, args = {:?}, body = {:?}, return_type = {:?}",
            func_name, arg_addresses, body, return_type
        );
        Ok(Value::Null)
    }

    fn eval_binary_increment(&mut self, lhs: &Box<Node>) -> R<Value, String> {
        let left_value = match self.execute_node(&lhs)? {
            Value::Number(v) => v,
            _ => serde_json::Number::from(-1),
        };
        let var = match lhs.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&var)
            .cloned()
            .or_else(|| self.context.global_context.get(&var).cloned());
        if let Some(variable) = variable_data {
            let result = left_value.as_i64().unwrap() + 1;
            self.memory_mgr
                .update_value(variable.address.clone(), result);
            Ok(Value::Number(result.into()))
        } else {
            Ok(Value::Null)
        }
    }
    fn eval_binary_decrement(&mut self, lhs: &Box<Node>) -> R<Value, String> {
        let left_value = match self.execute_node(&lhs)? {
            Value::Number(v) => v,
            _ => serde_json::Number::from(-1),
        };
        let var = match lhs.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&var)
            .cloned()
            .or_else(|| self.context.global_context.get(&var).cloned());
        if let Some(variable) = variable_data {
            let result = left_value.as_i64().unwrap() - 1;
            self.memory_mgr
                .update_value(variable.address.clone(), result);
            Ok(Value::Number(result.into()))
        } else {
            Ok(Value::Null)
        }
    }

    fn eval_binary_condition(&mut self, node: &Node) -> R<Value, String> {
        // 条件演算子の処理
        if let NodeValue::Eq(left, right)
        | NodeValue::Ne(left, right)
        | NodeValue::Lt(left, right)
        | NodeValue::Gt(left, right)
        | NodeValue::Le(left, right)
        | NodeValue::Ge(left, right) = &node.node_value()
        {
            let left_value = self.execute_node(left)?;
            let right_value = self.execute_node(right)?;

            match (&node.node_value(), left_value, right_value) {
                // 等しい (==)
                (NodeValue::Eq(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 == r_f64))
                }
                (NodeValue::Eq(_, _), Value::String(l), Value::String(r)) => {
                    Ok(Value::Bool(l == r))
                }
                // 等しくない (!=)
                (NodeValue::Ne(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 != r_f64))
                }
                (NodeValue::Ne(_, _), Value::String(l), Value::String(r)) => {
                    Ok(Value::Bool(l != r))
                }
                // 小なり (<)
                (NodeValue::Lt(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 < r_f64))
                }
                // 大なり (>)
                (NodeValue::Gt(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 > r_f64))
                }
                // 以下 (<=)
                (NodeValue::Le(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 <= r_f64))
                }
                // 以上 (>=)
                (NodeValue::Ge(_, _), Value::Number(l), Value::Number(r)) => {
                    let l_f64 = l.as_f64().ok_or("Failed to convert left number to f64")?;
                    let r_f64 = r.as_f64().ok_or("Failed to convert right number to f64")?;
                    Ok(Value::Bool(l_f64 >= r_f64))
                }

                _ => Err("Unsupported operation or mismatched types in condition".to_string()),
            }
        } else {
            Err("Unsupported node value".to_string())
        }
    }

    fn eval_binary_op(&mut self, node: &Node) -> R<Value, String> {
        if let NodeValue::Add(lhs, rhs)
        | NodeValue::Sub(lhs, rhs)
        | NodeValue::Mul(lhs, rhs)
        | NodeValue::Div(lhs, rhs)
        | NodeValue::AddAssign(lhs, rhs)
        | NodeValue::SubAssign(lhs, rhs)
        | NodeValue::MulAssign(lhs, rhs)
        | NodeValue::DivAssign(lhs, rhs) = &node.node_value()
        {
            let left_value = self.execute_node(lhs)?;
            let right_value = self.execute_node(rhs)?;

            match (&node.node_value(), left_value.clone(), right_value.clone()) {
                (NodeValue::Add(_, _), Value::Number(l), Value::Number(r)) => {
                    if l.is_i64() && r.is_i64() {
                        let result = l.as_i64().unwrap() + r.as_i64().unwrap();
                        info!("Add: {} + {}", l, r);
                        Ok(Value::Number(serde_json::Number::from(result)))
                    } else {
                        info!("Add: {} + {}", l, r);
                        Ok(Value::Number(
                            serde_json::Number::from_f64(l.as_f64().unwrap() + r.as_f64().unwrap())
                                .unwrap(),
                        ))
                    }
                }
                (NodeValue::Add(_, _), Value::String(l), Value::String(r)) => {
                    let result = l.clone() + &r.clone();
                    info!("Add: \"{}\" + \"{}\"", l, r);
                    Ok(Value::String(result))
                }

                (NodeValue::Sub(_, _), Value::Number(l), Value::Number(r)) => {
                    if l.is_i64() && r.is_i64() {
                        let result = l.as_i64().unwrap() - r.as_i64().unwrap();
                        info!("Sub: {} - {}", l, r);
                        Ok(Value::Number(serde_json::Number::from(result)))
                    } else {
                        info!("Sub: {} - {}", l, r);
                        Ok(Value::Number(
                            serde_json::Number::from_f64(l.as_f64().unwrap() - r.as_f64().unwrap())
                                .unwrap(),
                        ))
                    }
                }

                (NodeValue::Mul(_, _), Value::Number(l), Value::Number(r)) => {
                    if l.is_i64() && r.is_i64() {
                        let result = l.as_i64().unwrap() * r.as_i64().unwrap();
                        info!("Mul: {} * {}", l, r);
                        Ok(Value::Number(serde_json::Number::from(result)))
                    } else {
                        info!("Mul: {} * {}", l, r);
                        Ok(Value::Number(
                            serde_json::Number::from_f64(l.as_f64().unwrap() * r.as_f64().unwrap())
                                .unwrap(),
                        ))
                    }
                }

                (NodeValue::Div(_, _), Value::Number(l), Value::Number(r)) => {
                    if r.as_f64().unwrap() == 0.0 {
                        return Err(compile_error!(
                            "error",
                            self.current_node.clone().unwrap().1.line(),
                            self.current_node.clone().unwrap().1.column(),
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Division by zero: {:?} / {:?}",
                            left_value.clone(),
                            right_value.clone()
                        ));
                    }
                    if l.is_i64() && r.is_i64() {
                        let result = l.as_i64().unwrap() / r.as_i64().unwrap();
                        info!("Div: {} / {}", l, r);
                        Ok(Value::Number(serde_json::Number::from(result)))
                    } else {
                        info!("Div: {} / {}", l, r);
                        Ok(Value::Number(
                            serde_json::Number::from_f64(l.as_f64().unwrap() / r.as_f64().unwrap())
                                .unwrap(),
                        ))
                    }
                }

                (NodeValue::AddAssign(_, _), Value::Number(l), Value::Number(r)) => {
                    let var = match lhs.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    };

                    let variable_data = self
                        .context
                        .local_context
                        .get(&var)
                        .cloned()
                        .or_else(|| self.context.global_context.get(&var).cloned());
                    if let Some(mut variable) = variable_data {
                        // 可変性のチェック
                        if variable.is_mutable {
                            let result = l.as_i64().unwrap() + r.as_i64().unwrap();
                            self.memory_mgr.update_value(
                                variable.address.clone(),
                                Value::Number(result.into()),
                            );
                            variable.value = result.into();
                            if self.context.local_context.contains_key(&var) {
                                self.context.local_context.insert(var.clone(), variable);
                            } else {
                                self.context.global_context.insert(var.clone(), variable);
                            }
                            info!("AddAssign: {} + {}", l, r);
                            Ok(Value::Number(result.into()))
                        } else {
                            Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' is not mutable",
                                var
                            ))
                        }
                    } else {
                        Ok(Value::Null)
                    }
                }

                (NodeValue::SubAssign(_, _), Value::Number(l), Value::Number(r)) => {
                    let var = match lhs.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    };

                    let variable_data = self
                        .context
                        .local_context
                        .get(&var)
                        .cloned()
                        .or_else(|| self.context.global_context.get(&var).cloned());

                    if let Some(variable) = variable_data {
                        if variable.is_mutable {
                            let result = l.as_i64().unwrap() - r.as_i64().unwrap();

                            self.memory_mgr.update_value(
                                variable.address.clone(),
                                Value::Number(result.into()),
                            );
                            info!("SubAssign: {} + {}", l, r);
                            Ok(Value::Number(result.into()))
                        } else {
                            Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' is not mutable",
                                var
                            ))
                        }
                    } else {
                        Ok(Value::Null)
                    }
                }

                (NodeValue::MulAssign(_, _), Value::Number(l), Value::Number(r)) => {
                    let var = match lhs.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    };

                    let variable_data = self
                        .context
                        .local_context
                        .get(&var)
                        .cloned()
                        .or_else(|| self.context.global_context.get(&var).cloned());
                    if let Some(variable) = variable_data {
                        if variable.is_mutable {
                            let result = l.as_i64().unwrap() * r.as_i64().unwrap();

                            self.memory_mgr.update_value(
                                variable.address.clone(),
                                Value::Number(result.into()),
                            );
                            info!("MulAssign: {} + {}", l, r);
                            Ok(Value::Number(result.into()))
                        } else {
                            Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' is not mutable",
                                var
                            ))
                        }
                    } else {
                        Ok(Value::Null)
                    }
                }

                (NodeValue::DivAssign(_, _), Value::Number(l), Value::Number(r)) => {
                    let var = match lhs.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    };

                    let variable_data = self
                        .context
                        .local_context
                        .get(&var)
                        .cloned()
                        .or_else(|| self.context.global_context.get(&var).cloned());
                    if let Some(variable) = variable_data {
                        if variable.is_mutable {
                            let result = l.as_i64().unwrap() / r.as_i64().unwrap();
                            self.memory_mgr.update_value(
                                variable.address.clone(),
                                Value::Number(result.into()),
                            );
                            info!("DivAssign: {} + {}", l, r);
                            Ok(Value::Number(result.into()))
                        } else {
                            Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' is not mutable",
                                var
                            ))
                        }
                    } else {
                        Ok(Value::Null)
                    }
                }

                _ => Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line(),
                    self.current_node.clone().unwrap().1.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "operation failed: {:?} + {:?}",
                    left_value.clone(),
                    right_value.clone()
                )),
            }
        } else {
            Err("Unsupported node value".to_string())
        }
    }
    /*
        fn eval_if_statement(&mut self, condition: &Box<Node>, body: &Box<Node>) -> R<Value, String> {
            let condition = self.execute_node(&condition)?;
            let mut result = Value::Null;
            if let Value::Bool(value) = condition {
                if value {
                    result = self.execute_node(&body)?;
                }
            }
            Ok(result)
        }
    */

    fn eval_if_statement(&mut self, condition: &Box<Node>, body: &Box<Node>) -> R<Value, String> {
        let condition_result = self.execute_node(&condition)?;
        let mut result = Value::Null;

        if let Value::Bool(value) = condition_result {
            if value {
                result = self.execute_node(&body)?;
            } else if let Some(ref next_node) = condition.node_next() {
                match next_node.node_value() {
                    NodeValue::If(ref next_condition, ref next_body) => {
                        result = self.eval_if_statement(next_condition, next_body)?;
                    }
                    NodeValue::Else(ref else_body) => {
                        result = self.execute_node(&else_body)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(result)
    }
    fn eval_loop_statement(&mut self, body: &Box<Node>) -> R<Value, String> {
        let mut result = Value::Null;
        loop {
            result = self.execute_node(&body)?;
        }
        Ok(result)
    }
    fn eval_while_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> R<Value, String> {
        let mut result = Value::Null;
        loop {
            let condition_value = self.execute_node(&condition)?;
            if let Value::Bool(value) = condition_value {
                if value {
                    match self.execute_node(&body) {
                        Ok(val) => {
                            if val == Value::String("break".to_string()) {
                                break;
                            } else if val == Value::String("continue".to_string()) {
                                continue;
                            } else {
                                result = val;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    break;
                }
            } else {
                return Err("Condition must evaluate to a boolean".to_string());
            }
        }
        Ok(result)
    }

    fn eval_for_statement(
        &mut self,
        value: &Box<Node>,
        iterator: &Box<Node>,
        body: &Box<Node>,
    ) -> R<Value, String> {
        let mut result = Value::Null;

        // イテレータの評価
        let iter_value = self.execute_node(iterator)?;
        if let Value::Array(elements) = iter_value {
            for element in elements {
                // ループ変数に値を設定し、メモリを確保
                let element_address = self.memory_mgr.allocate(element.clone());
                let variable = Variable {
                    data_type: Value::String("void".to_string()), // 型推論を仮定
                    value: element.clone(),
                    address: element_address,
                    is_mutable: true, // 仮に可変とする
                    size: element.size(),
                };
                let var = match value.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };
                self.context.local_context.insert(var.clone(), variable);

                // ループボディの評価
                match self.execute_node(body) {
                    Ok(val) => {
                        if val == Value::String("break".to_string()) {
                            break;
                        } else if val == Value::String("continue".to_string()) {
                            continue;
                        } else {
                            result = val;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "The iterator is not an array",
            ));
        }

        Ok(result)
    }
    fn eval_primitive_type(&mut self, node: &Node) -> R<Value, String> {
        match &node.node_value() {
            NodeValue::Int(number) => Ok(Value::Number((*number).into())),
            NodeValue::Float(number) => {
                let n = Number::from_f64(*number).unwrap();
                Ok(Value::Number(n.into()))
            }
            NodeValue::String(s) => Ok(Value::String(s.clone())),
            NodeValue::Bool(b) => Ok(Value::Bool(*b)),
            NodeValue::Array(data_type, values) => self.eval_array(&data_type, &values),
            _ => Ok(Value::Null),
        }
    }
    // ノードを評価
    fn execute_node(&mut self, node: &Node) -> R<Value, String> {
        let mut result = Value::Null;
        let node_value = node.clone().node_value();

        //info!("global_contexts: {:?}", self.context.global_context.clone());
        //info!("local_contexts: {:?}", self.context.local_context.clone());
        //info!("used_context: {:?}", self.context.used_context.clone());
        //info!("current_node: {:?}", self.current_node.clone());

        match &node_value {
            NodeValue::Null => {
                result = Value::Null;
            }
            NodeValue::Range(start, max) => {
                let start_value = self.execute_node(start)?;
                let max_value = self.execute_node(max)?;
                let array: Vec<u64> =
                    (start_value.as_u64().unwrap()..=max_value.as_u64().unwrap()).collect();
                result = serde_json::json!(array);
            }
            NodeValue::Break => {
                result = Value::String("break".into());
            }
            NodeValue::Continue => {
                result = Value::String("continue".into());
            }
            NodeValue::Loop(body) => {
                result = self.eval_loop_statement(body)?;
            }
            NodeValue::If(condition, body) => {
                result = self.eval_if_statement(condition, body)?;
            }

            NodeValue::While(condition, body) => {
                result = self.eval_while_statement(condition, body)?;
            }
            NodeValue::For(value, iterator, body) => {
                result = self.eval_for_statement(value, iterator, body)?;
            }
            NodeValue::Eq(_, _)
            | NodeValue::Ne(_, _)
            | NodeValue::Lt(_, _)
            | NodeValue::Gt(_, _)
            | NodeValue::Le(_, _)
            | NodeValue::Ge(_, _)
            | NodeValue::And(_, _)
            | NodeValue::Or(_, _) => {
                result = self.eval_binary_condition(&node.clone())?;
            }

            NodeValue::Int(_)
            | NodeValue::Float(_)
            | NodeValue::String(_)
            | NodeValue::Bool(_)
            | NodeValue::Array(_, _) => {
                result = self.eval_primitive_type(&node.clone())?;
            }

            NodeValue::Include(file_name) => {
                result = self.eval_include(file_name)?;
            }
            NodeValue::MultiComment(content, (line, column)) => {
                self.eval_multi_comment(&content, &(*line, *column))?;
                return Ok(result);
            }
            NodeValue::SingleComment(content, (line, column)) => {
                self.eval_single_comment(&content, &(*line, *column))?;
                return Ok(result);
            }
            NodeValue::Block(block) => {
                result = self.eval_block(&block)?;
            }
            NodeValue::Assign(var_name, value, index) => {
                result = self.eval_assign(&node.clone(), &var_name, &value, &index)?;
            }

            NodeValue::Call(name, args, is_system) => {
                result = self.eval_call(name, args, is_system)?;
            }
            NodeValue::CallBackFunction(name, args, body, return_type, is_system) => {
                result = self.eval_callback_function(name, args, body, return_type, is_system)?;
            }
            NodeValue::Function(name, args, body, return_type, is_system) => {
                result = self.eval_function(name, args, body, return_type, is_system)?;
            }

            NodeValue::VariableDeclaration(
                var_name,
                data_type,
                value,
                is_local,
                is_mutable,
                is_reference,
            ) => {
                result = self.eval_variable_declaration(
                    &node.clone(),
                    var_name,
                    data_type,
                    value,
                    is_local,
                    is_mutable,
                    is_reference,
                )?;
            }

            NodeValue::TypeDeclaration(_type_name, _type) => {
                result = self.eval_type_declaration(_type_name, _type)?;
            }
            NodeValue::Variable(name) => {
                result = self.eval_variable(name)?;
            }

            NodeValue::Return(ret) => {
                result = self.eval_return(ret)?;
            }

            NodeValue::Increment(lhs) => {
                result = self.eval_binary_increment(lhs)?;
            }
            NodeValue::Decrement(lhs) => {
                result = self.eval_binary_decrement(lhs)?;
            }

            NodeValue::Add(_, _)
            | NodeValue::Sub(_, _)
            | NodeValue::Mul(_, _)
            | NodeValue::Div(_, _)
            | NodeValue::AddAssign(_, _)
            | NodeValue::SubAssign(_, _)
            | NodeValue::MulAssign(_, _)
            | NodeValue::DivAssign(_, _) => {
                result = self.eval_binary_op(&node.clone())?;
            }
            _ => {
                return Err(compile_error!(
                    "error",
                    node.line(),
                    node.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Unknown node value: {:?}",
                    node.node_value()
                ));
            }
        }
        Ok(result)
    }
}
