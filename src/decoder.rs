use crate::compile_error;
use crate::compile_group_error;

use crate::error::CompilerError;
use crate::lexer::{Lexer, Token};
use crate::parser::Node;
use crate::parser::Parser;
use crate::types::NodeValue;
use anyhow::Result as R;
use chrono::{DateTime, Local, Utc};
use hostname::get;
use indexmap::IndexMap;
use log::info;
use property_rs::Property;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use serde_json::{Number, Value};
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Add, Div, Mul, Sub};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use uuid::Uuid;
use whoami;

trait Size {
    fn size(&self) -> usize;
}

impl Size for Value {
    fn size(&self) -> usize {
        match self {
            Value::Null => 0,
            Value::Bool(_) => std::mem::size_of::<bool>(),
            Value::Number(n) => {
                if n.is_i64() {
                    std::mem::size_of::<i64>()
                } else if n.is_u64() {
                    std::mem::size_of::<u64>()
                } else if n.is_f64() {
                    std::mem::size_of::<f64>()
                } else if let Some(i) = n.as_i64() {
                    if i <= i32::MAX as i64 && i >= i32::MIN as i64 {
                        std::mem::size_of::<i32>()
                    } else {
                        std::mem::size_of::<i64>()
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= u32::MAX as u64 {
                        std::mem::size_of::<u32>()
                    } else {
                        std::mem::size_of::<u64>()
                    }
                } else if let Some(f) = n.as_f64() {
                    if f <= f32::MAX as f64 && f >= f32::MIN as f64 {
                        std::mem::size_of::<f32>()
                    } else {
                        std::mem::size_of::<f64>()
                    }
                } else {
                    0 // ここは適宜調整
                }
            }
            Value::String(s) => std::mem::size_of::<String>() + s.len(),
            Value::Array(arr) => {
                std::mem::size_of::<Vec<Value>>() + arr.iter().map(|v| v.size()).sum::<usize>()
            }
            Value::Object(obj) => {
                std::mem::size_of::<serde_json::Map<String, Value>>()
                    + obj.iter().map(|(k, v)| k.len() + v.size()).sum::<usize>()
            }
        }
    }
}

impl Clone for MemoryBlock {
    fn clone(&self) -> Self {
        // クローン処理。今回はidのみクローンし、valueはクローン不可のため新たに初期化
        MemoryBlock {
            id: self.id,
            value: Box::new(()), // クローンできないためデフォルトの空の値を持たせる
        }
    }
}
#[derive(Debug)]
struct MemoryBlock {
    id: Uuid,
    value: Box<dyn Any>,
}

impl MemoryBlock {
    // クローン可能な値を持つ場合のみクローンを許可
    pub fn clone_block(&self) -> Option<MemoryBlock> {
        if let Some(cloned_value) = self.value.downcast_ref::<String>() {
            Some(MemoryBlock {
                id: self.id,
                value: Box::new(cloned_value.clone()) as Box<dyn Any>,
            })
        } else {
            None // クローンできない場合はNoneを返す
        }
    }
}
// メモリの管理
#[derive(Debug, Clone)]
struct MemoryManager {
    pub heap: HashMap<Uuid, MemoryBlock>, // ヒープ(アドレス,値)
    pub free_list: Vec<Uuid>,
}

impl MemoryManager {
    fn new(heap_size: usize) -> Self {
        MemoryManager {
            heap: HashMap::new(),
            free_list: Vec::new(),
        }
    }
}

// 変数情報
#[derive(Debug, Clone)]
pub struct Variable {
    data_type: Value, // 型
    value: Value,     // 値
    address: Uuid,    // アドレス
    is_mutable: bool, // 可変性
    size: usize,      // サイズ
}
// コンテキスト
#[derive(Debug, Clone, Property)]
pub struct Context {
    pub local_context: IndexMap<String, Variable>, // ローカルスコープ
    pub global_context: IndexMap<String, Variable>, // グローバルスコープ
    pub type_context: IndexMap<String, String>,    // グローバルス型定義スコープ
    pub comment_lists: IndexMap<(usize, usize), Vec<String>>, // コメントリスト
    pub used_context: IndexMap<String, (usize, usize, bool)>, // 参照カウント(変数名,(行数,列数,参照カウント))
}
impl Context {
    fn new() -> Self {
        Context {
            local_context: IndexMap::new(),
            global_context: IndexMap::new(),
            type_context: IndexMap::new(),
            comment_lists: IndexMap::new(),
            used_context: IndexMap::new(),
        }
    }
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

        let nodes = Parser::from_parse(&tokens, file_name, file_content.clone())?;
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
    // 指定の型の値を確保してUUIDのアドレスを返す
    fn allocate<T: 'static + Any>(&mut self, value: T) -> Uuid {
        let id = if let Some(free_id) = self.memory_mgr.free_list.pop() {
            // 解放済みのブロックがあれば再利用
            free_id
        } else {
            Uuid::new_v4() // 新しいUUIDを生成
        };
        let block = MemoryBlock {
            id,
            value: Box::new(value),
        };
        self.memory_mgr.heap.insert(id, block);
        id // 割り当てたメモリのIDを返す
    }
    // 指定アドレス(UUID)のメモリを開放
    fn deallocate(&mut self, id: Uuid) {
        if self.memory_mgr.heap.remove(&id).is_some() {
            self.memory_mgr.free_list.push(id); // 解放されたメモリブロックをフリーリストに追加
        }
    }
    // 指定のアドレスの値を返す
    fn get_value<T: 'static + Any>(&self, id: Uuid) -> Option<&T> {
        self.memory_mgr
            .heap
            .get(&id)
            .and_then(|block| block.value.downcast_ref::<T>()) // IDから値を取得
    }
    // 指定のアドレスの値を更新
    fn update_value<T: 'static + Any>(&mut self, id: Uuid, new_value: T) -> bool {
        if let Some(block) = self.memory_mgr.heap.get_mut(&id) {
            block.value = Box::new(new_value); // 新しい値で更新
            true
        } else {
            false // 指定されたIDが見つからなかった場合
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
        self.allocate(array.clone());
        // 結果を返す
        Ok(Value::Array(array.clone()))
    }
    fn eval_assign(&mut self, var_name: &Box<Node>, value: &Box<Node>) -> R<Value, String> {
        let mut result = Value::Null;
        // ステートメントフラグのチェック
        /*
        if !self.current_node.clone().unwrap().1.is_statement() {
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
        */
        let name = match var_name.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

        // 変数のデータを一時変数にコピー
        let variable_data = self
            .context
            .local_context
            .get(&name)
            .cloned()
            .or_else(|| self.context.global_context.get(&name).cloned());

        if let Some(mut variable) = variable_data {
            // 可変性のチェックを追加
            if variable.is_mutable {
                let new_value = self.execute_node(&value)?;

                // 型チェックを追加
                self.check_type(&new_value, variable.data_type.as_str().unwrap_or(""))?;

                // 変数の値を更新
                self.update_value(variable.address.clone(), new_value.clone());
                variable.value = new_value.clone();

                if self.context.local_context.contains_key(&name) {
                    self.context.local_context.insert(name.clone(), variable);
                } else {
                    self.context.global_context.insert(name.clone(), variable);
                }
                info!("Assign: name = {:?}, new_value = {:?}", name, new_value);
                result = new_value.clone();
                Ok(new_value)
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

        if *is_system {
            match name.as_str() {
                "exit" => {
                    if args.len() != 1 {
                        return Err("exit expects exactly one argument".into());
                    }
                    let status = match self.execute_node(&args[0])? {
                        Value::Number(n) => n.as_i64().ok_or("exit expects a positive integer")?,
                        _ => return Err("exit expects a number as the status".into()),
                    };
                    std::process::exit(status.try_into().unwrap());
                }
                "args" => {
                    if !args.is_empty() {
                        return Err("args expects no arguments".into());
                    }
                    let args: Vec<String> = std::env::args().collect();
                    let value: Value = Value::Array(args.into_iter().map(Value::String).collect());
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
                    let command_args = if evaluated_args.len() > 1 {
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
                            _ => {
                                return Err(
                                    "cmd expects the second argument to be an array of strings"
                                        .into(),
                                )
                            }
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
        let func_info = self.get_value::<Value>(func_address).unwrap();
        let _args = func_info["args"].clone();
        let _body = func_info["body"].clone();
        let body: Node = serde_json::from_value(_body).unwrap();
        let return_type = func_info["return_type"].clone();

        for (arg, value) in _args.as_array().unwrap().iter().zip(&evaluated_args) {
            let arg_name = arg["name"].as_str().unwrap();
            let arg_type = arg["type"].clone();
            let index = self.allocate(value.clone());
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
            .filter(|node| node.node_value() != NodeValue::Empty)
            .collect::<Vec<_>>();
        for body in b {
            result = self.execute_node(&body)?;
        }

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

        let func_index = self.allocate(func_name.clone());

        for (i, (data_type, arg_name)) in args.iter().enumerate() {
            arg_addresses.push(serde_json::json!({"name": arg_name.clone(),"type": data_type}));
        }

        // 関数の情報をシリアライズしてヒープに格納
        let func_info = serde_json::json!({
            "args": arg_addresses,
            "body": body,
            "return_type": return_type,
        });
        let func_info_index = self.allocate(func_info.clone());

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
    fn eval_variable_declaration(
        &mut self,
        var_name: &Box<Node>,
        data_type: &Box<Node>,
        value: &Box<Node>,
        is_local: &bool,
        is_mutable: &bool,
        is_reference: &bool,
    ) -> R<Value, String> {
        let mut result = Value::Null;
        // ステートメントフラグのチェック
        /*
        if !self.current_node.clone().unwrap().1.is_statement() {
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
        */
        //info!("is_reference: {:?}", is_reference);
        let name = match var_name.node_value() {
            NodeValue::Variable(v) => v,
            _ => String::new(),
        };

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

            v_type = if let NodeValue::Empty = data_type.node_value() {
                let _value = self.execute_node(&value)?;
                v_value = _value.clone(); // ここでv_valueを設定
                Value::String(self.infer_type(&_value))
            } else {
                let v = match data_type.node_value() {
                    NodeValue::DataType(v_type) => match v_type.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    },
                    _ => String::new(),
                };
                v_value = if let NodeValue::Empty = value.node_value() {
                    Value::Number(serde_json::Number::from(0))
                } else {
                    let _value = self.execute_node(&value)?;
                    self.check_type(&_value, v_type.as_str().unwrap_or(""))?
                };
                Value::String(v.into())
            };
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
                        let _address = self.allocate(v_value.clone());
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
            address = self.allocate(v_value.clone());
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

        info!("VariableDeclaration: name = {:?}, data_type = {:?}, value = {:?}, address = {:?} is_local: {}", name, v_type, v_value, address,is_local);
        result = v_value.clone();
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

        let func_index = self.allocate(func_name.clone());

        for (i, (data_type, arg_name)) in args.iter().enumerate() {
            arg_addresses.push(serde_json::json!({"name": arg_name.clone(),"type": data_type}));
        }

        // 関数の情報をシリアライズしてヒープに格納
        let func_info = serde_json::json!({
            "args": arg_addresses,
            "body": body,
            "return_type": return_type,
        });
        let func_info_index = self.allocate(func_info.clone());

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
            self.update_value(variable.address.clone(), result);
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
            self.update_value(variable.address.clone(), result);
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
                            self.update_value(
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

                            self.update_value(
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

                            self.update_value(
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
                            self.update_value(
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
    fn eval_primitive_type(&mut self, node: &Node) -> R<Value, String> {
        match &node.node_value() {
            NodeValue::Int(number) => Ok(Value::Number((*number).into())),
            NodeValue::Float(number) => {
                let n = Number::from_f64(*number).unwrap();
                Ok(Value::Number(n.into()))
            }
            NodeValue::String(s) => Ok(Value::String(s.clone())),
            NodeValue::Bool(b) => Ok(Value::Bool(*b)),
            _ => Ok(Value::Null),
        }
    }
    // ノードを評価
    fn execute_node(&mut self, node: &Node) -> R<Value, String> {
        let result = Value::Null;
        let node_value = node.clone().node_value();

        //info!("global_contexts: {:?}", self.context.global_context.clone());
        //info!("local_contexts: {:?}", self.context.local_context.clone());
        //info!("used_context: {:?}", self.context.used_context.clone());
        //info!("current_node: {:?}", self.current_node.clone());

        match &node_value {
            NodeValue::If(condition, body) => self.eval_if_statement(condition, body),
            NodeValue::Eq(_, _)
            | NodeValue::Ne(_, _)
            | NodeValue::Lt(_, _)
            | NodeValue::Gt(_, _)
            | NodeValue::Le(_, _)
            | NodeValue::Ge(_, _)
            | NodeValue::And(_, _)
            | NodeValue::Or(_, _) => self.eval_binary_condition(&node.clone()),

            NodeValue::Int(_) | NodeValue::Float(_) | NodeValue::String(_) | NodeValue::Bool(_) => {
                self.eval_primitive_type(&node.clone())
            }

            NodeValue::Include(file_name) => self.eval_include(file_name),
            NodeValue::Empty => Ok(result),
            NodeValue::MultiComment(content, (line, column)) => {
                self.eval_multi_comment(&content, &(*line, *column))
            }
            NodeValue::SingleComment(content, (line, column)) => {
                self.eval_single_comment(&content, &(*line, *column))
            }

            NodeValue::Array(data_type, values) => self.eval_array(&data_type, &values),
            NodeValue::Block(block) => self.eval_block(&block),
            NodeValue::Assign(var_name, value) => self.eval_assign(&var_name, &value),

            NodeValue::Call(name, args, is_system) => self.eval_call(name, args, is_system),
            NodeValue::CallBackFunction(name, args, body, return_type, is_system) => {
                self.eval_callback_function(name, args, body, return_type, is_system)
            }
            NodeValue::Function(name, args, body, return_type, is_system) => {
                self.eval_function(name, args, body, return_type, is_system)
            }

            NodeValue::VariableDeclaration(
                var_name,
                data_type,
                value,
                is_local,
                is_mutable,
                is_reference,
            ) => self.eval_variable_declaration(
                var_name,
                data_type,
                value,
                is_local,
                is_mutable,
                is_reference,
            ),

            NodeValue::TypeDeclaration(_type_name, _type) => {
                self.eval_type_declaration(_type_name, _type)
            }
            NodeValue::Variable(name) => self.eval_variable(name),

            NodeValue::Return(ret) => self.eval_return(ret),

            NodeValue::Increment(lhs) => self.eval_binary_increment(lhs),
            NodeValue::Decrement(lhs) => self.eval_binary_decrement(lhs),

            NodeValue::Add(_, _)
            | NodeValue::Sub(_, _)
            | NodeValue::Mul(_, _)
            | NodeValue::Div(_, _)
            | NodeValue::AddAssign(_, _)
            | NodeValue::SubAssign(_, _)
            | NodeValue::MulAssign(_, _)
            | NodeValue::DivAssign(_, _) => self.eval_binary_op(&node.clone()),

            _ => Err(compile_error!(
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
            )),
        }
    }
}
