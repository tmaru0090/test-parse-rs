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
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Add, Div, Mul, Sub};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use whoami;
#[derive(Debug, Clone)]
pub struct Variable {
    data_type: Value,
    value: Value,
    address: usize,
    is_mutable: bool,
    size: usize,
}

#[derive(Debug, Clone, Property)]
pub struct Context {
    pub local_context: IndexMap<String, Variable>,
    pub global_context: IndexMap<String, Variable>,
    pub type_context: IndexMap<String, String>,
    pub comment_lists: IndexMap<(usize, usize), Vec<String>>,
    pub used_context: IndexMap<String, (usize, usize, bool)>,
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

#[derive(Debug, Clone)]
struct MemoryManager {
    pub heap: Vec<u8>,
    pub free_list: Vec<usize>,
}

impl MemoryManager {
    fn new(heap_size: usize) -> Self {
        MemoryManager {
            heap: vec![0; heap_size],
            free_list: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Property)]
pub struct Decoder {
    #[property(get)]
    ast_map: IndexMap<String, Vec<Box<Node>>>,
    #[property(get)]
    memory_mgr: MemoryManager,
    #[property(get)]
    context: Context,
    #[property(get)]
    file_contents: IndexMap<String, String>,
    #[property(get)]
    current_node: Option<(String, Box<Node>)>,

    #[property(get)]
    generated_ast_file: bool,

    #[property(get)]
    generated_error_log_file: bool,

    #[property(get)]
    measure_decode_time: bool,

    #[property(get)]
    decode_time: f32,
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
    pub fn add_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        self.ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(self)
    }
    pub fn load_script(file_name: &str) -> R<Self, String> {
        let mut ast_map: IndexMap<String, Vec<Box<Node>>> = IndexMap::new();
        let file_content = std::fs::read_to_string(file_name)
            .map_err(|e| e.to_string())
            .expect("Failed to script file");

        let tokens = Lexer::from_tokenize(file_name, file_content.clone())?;

        let nodes = Parser::from_parse(&tokens, file_name, file_content.clone())?;

        //info!("tokens: {:?}", tokens.clone());
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
        }
    }
    fn allocate(&mut self, size: usize) -> R<usize, String> {
        if let Some(index) = self.memory_mgr.free_list.pop() {
            Ok(index)
        } else {
            let index = self.memory_mgr.heap.len();
            // ヒープのサイズが不足している場合、拡張する
            if index + size > self.memory_mgr.heap.len() {
                let new_capacity = (index + size).max(self.memory_mgr.heap.len() * 2);
                self.memory_mgr.heap.resize(new_capacity, 0);
            }
            self.memory_mgr.heap.resize(index + size, 0);
            Ok(index)
        }
    }

    fn deallocate(&mut self, index: usize) {
        self.memory_mgr.free_list.push(index);
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
    fn allocate_and_copy_to_heap(&mut self, serialized_value: Vec<u8>) -> R<usize, String> {
        let (file_name, node) = if let Some((file_name, node)) = self.current_node.clone() {
            (file_name, node)
        } else {
            (String::new(), Box::new(Node::default()))
        };
        let index = self.allocate(serialized_value.len())?;
        if index + serialized_value.len() > self.memory_mgr.heap.len() {
            return Err(compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &file_name,
                &self.file_contents.get(&file_name).unwrap(),
                "Heap overflow: trying to write {} bytes at index {}, but heap size is {}",
                serialized_value.len(),
                index,
                self.memory_mgr.heap.len()
            ));
        }
        self.memory_mgr.heap[index..index + serialized_value.len()]
            .copy_from_slice(&serialized_value);
        Ok(index)
    }

    fn copy_to_heap(&mut self, address: usize, serialized_value: Vec<u8>) -> R<(), String> {
        let (file_name, node) = if let Some((file_name, node)) = self.current_node.clone() {
            (file_name, node)
        } else {
            (String::new(), Box::new(Node::default()))
        };

        if address + serialized_value.len() > self.memory_mgr.heap.len() {
            return Err(compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &file_name,
                &self.file_contents.get(&file_name).unwrap(),
                "Heap overflow: trying to write {} bytes at address {}, but heap size is {}",
                serialized_value.len(),
                address,
                self.memory_mgr.heap.len()
            ));
        }

        self.memory_mgr.heap[address..address + serialized_value.len()]
            .copy_from_slice(&serialized_value);
        Ok(())
    }
    pub fn read_from_heap(&self, address: usize, size: usize) -> Result<Vec<u8>, String> {
        if address + size > self.memory_mgr.heap.len() {
            return Err(format!(
                "Heap overflow: trying to read {} bytes at address {}, but heap size is {}",
                size,
                address,
                self.memory_mgr.heap.len()
            ));
        }

        Ok(self.memory_mgr.heap[address..address + size].to_vec())
    }

    fn deserialize_value(&self, v_type: &str, data: &[u8]) -> Result<Value, String> {
        match v_type {
            "i32" => {
                if data.len() != 4 {
                    return Err("Invalid data length for i32".to_string());
                }
                let num = i32::from_le_bytes(data.try_into().unwrap());
                Ok(Value::Number(num.into()))
            }
            "i64" => {
                if data.len() != 8 {
                    return Err("Invalid data length for i64".to_string());
                }
                let num = i64::from_le_bytes(data.try_into().unwrap());
                Ok(Value::Number(num.into()))
            }
            "f32" => {
                if data.len() != 4 {
                    return Err("Invalid data length for f32".to_string());
                }
                let num = f32::from_le_bytes(data.try_into().unwrap());
                Ok(Value::Number(
                    serde_json::Number::from_f64(num as f64).unwrap(),
                ))
            }
            "f64" => {
                if data.len() != 8 {
                    return Err("Invalid data length for f64".to_string());
                }
                let num = f64::from_le_bytes(data.try_into().unwrap());
                Ok(Value::Number(serde_json::Number::from_f64(num).unwrap()))
            }
            "string" => {
                let s = String::from_utf8(data.to_vec()).map_err(|e| e.to_string())?;
                Ok(Value::String(s))
            }
            "array" => {
                // 配列の各要素の型を指定する必要があります
                // ここでは仮に "element_type" としていますが、適切な型を指定してください
                let mut elements = Vec::new();
                let element_size = 4; // 仮のサイズ、適切なサイズを指定してください
                for chunk in data.chunks(element_size) {
                    elements.push(self.deserialize_value("element_type", chunk)?);
                }
                Ok(Value::Array(elements))
            }
            _ => serde_json::from_slice(data).map_err(|e| e.to_string()),
        }
    }
    fn serialize_value(&self, v_type: &str, v_value: &Value) -> Vec<u8> {
        match v_type {
            "i32" => {
                let num = v_value.as_i64().unwrap() as i32;
                num.to_le_bytes().to_vec()
            }
            "i64" => {
                let num = v_value.as_i64().unwrap();
                num.to_le_bytes().to_vec()
            }
            "f32" => {
                let num = v_value.as_f64().unwrap() as f32;
                num.to_le_bytes().to_vec()
            }
            "f64" => {
                let num = v_value.as_f64().unwrap();
                num.to_le_bytes().to_vec()
            }
            "string" => {
                if let Value::String(ref s) = v_value {
                    s.as_bytes().to_vec()
                } else {
                    vec![]
                }
            }
            "array" => {
                if let Value::Array(ref arr) = v_value {
                    arr.iter()
                        .flat_map(|elem| self.serialize_value(v_type, elem))
                        .collect()
                } else {
                    vec![]
                }
            }
            _ => serde_json::to_vec(v_value).unwrap(),
        }
    }
    fn get_value_from_heap(
        &self,
        heap: &[u8],
        index: usize,
        value_size: usize,
    ) -> Result<Value, String> {
        let (file_name, node) = if let Some((file_name, node)) = self.current_node.clone() {
            (file_name, node)
        } else {
            (String::new(), Box::new(Node::default()))
        };

        if index + value_size <= heap.len() {
            let slice = &heap[index..index + value_size];

            // スライスを数値または文字列に変換
            let v_value = match value_size {
                1 => Value::Number(slice[0].into()), // 1バイトの場合、数値として扱う
                4 => {
                    // 4バイトの場合、i32またはf32として扱う
                    if let Ok(num) = slice.try_into().map(i32::from_le_bytes) {
                        Value::Number(num.into())
                    } else {
                        let num = f32::from_le_bytes(slice.try_into().unwrap());
                        Value::Number(serde_json::Number::from_f64(num as f64).unwrap())
                    }
                }
                8 => {
                    // 8バイトの場合、i64またはf64として扱う
                    if let Ok(num) = slice.try_into().map(i64::from_le_bytes) {
                        Value::Number(num.into())
                    } else {
                        let num = f64::from_le_bytes(slice.try_into().unwrap());
                        Value::Number(serde_json::Number::from_f64(num).unwrap())
                    }
                }
                _ => Value::String(String::from_utf8_lossy(slice).to_string()), // その他の場合、文字列として扱う
            };

            Ok(v_value)
        } else {
            Err(compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &file_name,
                &self.file_contents.get(&file_name).unwrap(),
                "Index out of range: {} + {} > {}",
                index,
                value_size,
                heap.len()
            ))
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
        for (file_name, nodes) in self.ast_map() {
            self.current_node = Some((file_name.clone(), Box::new(Node::default())));
            let content = std::fs::read_to_string(file_name.clone()).map_err(|e| e.to_string())?;
            self.file_contents.insert(file_name.clone(), content);
            for node in nodes {
                self.current_node = Some((file_name.clone(), node.clone()));
                value = self.execute_node(&node)?;
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

    fn execute_node(&mut self, node: &Node) -> R<Value, String> {
        let mut result = Value::Null;
        //info!("global_contexts: {:?}", self.context.global_context.clone());
        //info!("local_contexts: {:?}", self.context.local_context.clone());
        //info!("used_context: {:?}", self.context.used_context.clone());
        //info!("current_node: {:?}", self.current_node.clone());

        match &node.node_value() {
            NodeValue::Include(file_name) => {
                self.add_ast_from_file(file_name)?;
                let ast_map = self.ast_map();
                let nodes = ast_map.get(file_name).unwrap();
                for node in nodes {
                    self.execute_node(&node)?;
                }
                Ok(Value::Null)
            }

            NodeValue::If(condition, body) => {
                // 条件を評価
                let cond_value = self.execute_node(&condition)?;
                // 条件が真の場合、ボディを実行
                if let Value::Bool(true) = cond_value {
                    self.execute_node(&body)?;
                }
                Ok(Value::Null)
            }
            NodeValue::For(value, array, body) => {
                // arrayが配列であることを確認
                if let Value::Array(ref arr) = self.execute_node(&array)? {
                    for elem in arr {
                        // valueにarrayの要素をセット
                        let variable = Variable {
                            data_type: elem.clone(),
                            value: elem.clone(),
                            address: 0, // アドレスは後で設定
                            is_mutable: false,
                            size: 0,
                        };
                        let value_name = match value.node_value() {
                            NodeValue::Variable(v) => v,
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
                                    "Expected a variable for the loop value"
                                ))
                            }
                        };

                        // 変数をヒープに追加
                        let serialized_value = self.serialize_value(
                            variable.data_type.as_str().unwrap_or(""),
                            &variable.value,
                        );
                        let index = self.allocate_and_copy_to_heap(serialized_value)?;
                        let mut temp_variable = variable.clone();
                        temp_variable.address = index;

                        self.context
                            .global_context
                            .insert(value_name.clone(), temp_variable.clone());

                        // bodyを実行
                        self.execute_node(&body)?;
                        // valueをグローバルスコープから削除
                        self.context.global_context.swap_remove(&value_name);
                    }
                    Ok(Value::Null)
                } else {
                    Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Expected an array for the For loop"
                    ))
                }
            }
            NodeValue::Array(data_type, values) => {
                // 型を評価
                let v_type = match data_type.node_value() {
                    NodeValue::DataType(d) => self.execute_node(&d)?,
                    _ => Value::Null,
                };

                // 各値を評価し、型チェックを行う
                let mut evaluated_values = Vec::new();
                for value in values {
                    let v_value = self.execute_node(&*value)?;
                    //self.check_type(&v_value, v_type.as_str().unwrap_or(""))?;
                    evaluated_values.push(v_value);
                }

                // 配列の各要素をシリアライズしてヒープに連続してコピー
                let mut serialized_array = Vec::new();
                for value in &evaluated_values {
                    let serialized_value =
                        self.serialize_value(v_type.as_str().unwrap_or(""), value);
                    serialized_array.extend(serialized_value);
                }

                // 配列全体をヒープにコピー
                let index = self.allocate(serialized_array.len())?;
                self.copy_to_heap(index, serialized_array.clone())?;

                info!(
                    "serialized_array: {:?} index: {:?}",
                    serialized_array, index
                );

                // 結果を返す
                Ok(Value::Array(evaluated_values))
            }
            NodeValue::Empty => Ok(result),
            NodeValue::MultiComment(content, (line, column)) => {
                self.context
                    .comment_lists
                    .insert((*line, *column), content.clone().to_vec());
                info!("MultiComment added at line {}, column {}", line, column);
                Ok(result)
            }
            NodeValue::SingleComment(content, (line, column)) => {
                self.context
                    .comment_lists
                    .insert((*line, *column), vec![content.clone()]);
                info!("SingleComment added at line {}, column {}", line, column);
                Ok(result)
            }

            NodeValue::Block(block) => {
                let mut r = Value::Null;
                let initial_local_context = self.context.local_context.clone(); // 現在のローカルコンテキストを保存
                for b in block {
                    r = self.execute_node(b)?;
                    info!("block: {:?}", b.clone());
                }
                self.context.local_context = initial_local_context; // ブロックの処理が終わったらローカルコンテキストを元に戻す
                Ok(r)
            }
            NodeValue::Assign(var_name, value) => {
                // ステートメントフラグのチェック
                if !node.is_statement() {
                    return Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
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

                        let serialized_value = self
                            .serialize_value(variable.data_type.as_str().unwrap_or(""), &new_value);
                        self.copy_to_heap(variable.address, serialized_value)?;

                        // 変数の値を更新
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
                            node.line(),
                            node.column(),
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
                        node.line(),
                        node.column(),
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

            NodeValue::Call(name, args, is_system) => {
                if *is_system {
                    match name.as_str() {
                        "get_file_metadata" => {
                            if args.len() != 1 {
                                return Err("get_file_metadata expects exactly one argument".into());
                            }
                            let file_path = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err(
                                        "get_file_metadata expects a string as the file path"
                                            .into(),
                                    )
                                }
                            };
                            let metadata = std::fs::metadata(file_path).unwrap();
                            let file_size = metadata.len();
                            let created = metadata
                                .created()
                                .unwrap()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            let modified = metadata
                                .modified()
                                .unwrap()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            let result = format!(
                "Size: {} bytes, Created: {} seconds since UNIX epoch, Modified: {} seconds since UNIX epoch",
                file_size, created, modified
            );
                            return Ok(Value::String(result));
                        }
                        "get_hostname" => {
                            if !args.is_empty() {
                                return Err("get_hostname expects no arguments".into());
                            }
                            let hostname = hostname::get().unwrap().to_string_lossy().to_string();
                            return Ok(Value::String(hostname));
                        }
                        "get_os" => {
                            if !args.is_empty() {
                                return Err("get_os expects no arguments".into());
                            }
                            let os = std::env::consts::OS;
                            return Ok(Value::String(os.to_string()));
                        }
                        "get_username" => {
                            if !args.is_empty() {
                                return Err("get_user expects no arguments".into());
                            }
                            let user = whoami::username();
                            return Ok(Value::String(user));
                        }
                        "get_env" => {
                            if args.len() != 1 {
                                return Err("get_env expects exactly one argument".into());
                            }
                            let var_name = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err(
                                        "get_env expects a string as the variable name".into()
                                    )
                                }
                            };
                            let var_value = std::env::var(&var_name).unwrap_or_default();
                            return Ok(Value::String(var_value));
                        }

                        "now" => {
                            if args.len() != 1 {
                                return Err("now expects exactly one argument".into());
                            }
                            let format = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => return Err("sleep expects a number as the duration".into()),
                            };
                            let now = Local::now();
                            return Ok(Value::String(now.format(&format).to_string()));
                        }

                        "sleep" => {
                            if args.len() != 1 {
                                return Err("sleep expects exactly one argument".into());
                            }
                            let duration = match self.execute_node(&args[0])? {
                                Value::Number(n) => {
                                    n.as_u64().ok_or("sleep expects a positive integer")?
                                }
                                _ => return Err("sleep expects a number as the duration".into()),
                            };
                            sleep(Duration::from_secs(duration));
                        }
                        "print" => {
                            for a in args {
                                let _value = self.execute_node(a)?;
                                let value = match _value {
                                    Value::String(v) => v,
                                    _ => format!("{}", _value),
                                };
                                print!("{}", value);
                            }
                        }
                        "println" => {
                            for a in args {
                                let _value = self.execute_node(a)?;
                                let value = match _value {
                                    Value::String(v) => v,
                                    _ => format!("{}", _value),
                                };
                                println!("{}", value);
                            }
                        }
                        "read_file" => {
                            if args.len() != 1 {
                                return Err("read_file expects exactly one argument".into());
                            }
                            let file_name = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err("read_file expects a string as the file name".into())
                                }
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
                                _ => {
                                    return Err(
                                        "write_file expects a string as the file name".into()
                                    )
                                }
                            };
                            let content = match self.execute_node(&args[1])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err("write_file expects a string as the content".into())
                                }
                            };
                            let mut file = File::create(file_name).unwrap();
                            file.write_all(content.as_bytes()).unwrap();
                        }

                        "cmd" => {
                            if args.len() < 1 {
                                return Err("execute_command expects at least one argument".into());
                            }
                            let command = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err(
                                        "execute_command expects a string as the command".into()
                                    )
                                }
                            };
                            let command_args: Vec<String> = args[1..]
                                .iter()
                                .map(|arg| match self.execute_node(arg) {
                                    Ok(Value::String(v)) => Ok(v),
                                    Ok(v) => Ok(format!("{}", v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<Vec<String>, _>>()?;
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

                        "set_env" => {
                            if args.len() != 2 {
                                return Err("set_env expects exactly two arguments".into());
                            }
                            let var_name = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err(
                                        "set_env expects a string as the variable name".into()
                                    )
                                }
                            };
                            let var_value = match self.execute_node(&args[1])? {
                                Value::String(v) => v,
                                _ => {
                                    return Err(
                                        "set_env expects a string as the variable value".into()
                                    )
                                }
                            };
                            std::env::set_var(var_name, var_value);
                        }
                        "current_dir" => {
                            let path = std::env::current_dir()
                                .expect("カレントディレクトリの取得に失敗しました");
                            return Ok(Value::String(path.to_string_lossy().to_string()));
                        }
                        "change_dir" => {
                            if args.len() != 1 {
                                return Err("change_dir expects exactly one argument".into());
                            }
                            let path = match self.execute_node(&args[0])? {
                                Value::String(v) => v,
                                _ => return Err("change_dir expects a string as the path".into()),
                            };
                            std::env::set_current_dir(path)
                                .expect("カレントディレクトリの変更に失敗しました");
                        }
                        _ => return Err(format!("Unknown function: {}", name)),
                    }
                }

                let func_name = name; // すでに String 型なのでそのまま使う

                // 関数が定義されているかチェック
                let func = {
                    let global_context = &self.context.global_context;
                    global_context
                        .get(func_name.as_str())
                        .cloned()
                        .ok_or_else(|| {
                            compile_error!(
                                "error",
                                node.line(),
                                node.column(),
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

                let mut arg_addresses = Vec::new();

                // 引数をシリアライズしてヒープに格納
                for arg in args.iter() {
                    let serialized_value = serde_json::to_vec(arg).map_err(|e| e.to_string())?;
                    let index = self.allocate_and_copy_to_heap(serialized_value)?;
                    arg_addresses.push(index);
                }

                // 関数情報を取得
                let (func_info_index, func_info_size) = {
                    let func_value = &func.value;
                    match func_value {
                        Value::Number(n) => (n.as_u64().unwrap() as usize, func.size),
                        _ => return Err("Invalid function index".into()),
                    }
                };

                let func_info_bytes = self.read_from_heap(func_info_index, func_info_size)?;
                let func_info: serde_json::Value =
                    serde_json::from_slice(&func_info_bytes).map_err(|e| e.to_string())?;

                // 関数の引数と本体を取得
                let func_args = func_info["args"].as_array().unwrap();
                let func_body: Node =
                    serde_json::from_value(func_info["body"].clone()).map_err(|e| e.to_string())?;
                // 引数をローカルコンテキストに設定
                for (i, arg_address) in arg_addresses.iter().enumerate() {
                    let arg_name = &func_args[i]["name"];
                    let arg_type_node: Node = serde_json::from_value(func_args[i]["type"].clone())
                        .map_err(|e| e.to_string())?;

                    // 型名を取得
                    let arg_type = match arg_type_node.node_value() {
                        NodeValue::Variable(ref type_name) => type_name.clone(),

                        NodeValue::DataType(ref data_type_node) => {
                            match data_type_node.node_value() {
                                NodeValue::Variable(ref type_name) => type_name.clone(),
                                _ => return Err("Invalid data type".into()),
                            }
                        }
                        _ => return Err("Invalid data type node".into()),
                    };

                    // 引数の型に応じてデシリアライズ
                    let bytes = self.read_from_heap(*arg_address, 4)?; // 適切なサイズを指定
                    let value = self.deserialize_value(&arg_type, &bytes)?;

                    self.context.local_context.insert(
                        arg_name.as_str().unwrap().to_string(),
                        Variable {
                            value,
                            data_type: Value::String("Argument".into()),
                            address: *arg_address,
                            is_mutable: false,
                            size: 0, // 引数のサイズはここでは不要
                        },
                    );
                }
                let func_result = self.execute_node(&func_body)?;
                let _func_return_type: Node =
                    serde_json::from_value(func_info["return_type"].clone()).unwrap();
                let func_return_type = match _func_return_type.node_value() {
                    NodeValue::ReturnType(v) => match v.node_value() {
                        NodeValue::Variable(v) => v,
                        _ => String::new(),
                    },
                    _ => String::new(),
                };
                self.check_type(&func_result, &func_return_type)?;
                Ok(func_result.clone())
            }

            NodeValue::Function(name, args, body, return_type, is_system) => {
                let func_name = name; // すでに String 型なのでそのまま使う
                                      // 関数がすでに定義されているかチェック
                if self.context.global_context.contains_key(func_name.as_str()) {
                    return Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Function '{}' is already defined",
                        func_name
                    ));
                }

                // 関数の引数を連続したアドレスに設定
                let mut arg_addresses = Vec::new();
                for (i, (data_type, arg_name)) in args.iter().enumerate() {
                    let serialized_value =
                        serde_json::to_vec(&Value::Number(serde_json::Number::from(i)))
                            .map_err(|e| e.to_string())?;
                    let index = self.allocate_and_copy_to_heap(serialized_value)?;
                    arg_addresses.push(serde_json::json!({"name": arg_name.clone(), "address": index, "type": data_type}));
                }

                // 関数の情報をシリアライズしてヒープに格納
                let func_info = serde_json::json!({
                    "args": arg_addresses,
                    "body": body,
                    "return_type": return_type,
                });
                let serialized_func_info =
                    serde_json::to_vec(&func_info).map_err(|e| e.to_string())?;
                let func_info_size = serialized_func_info.len(); // シリアライズされたデータのサイズを記録

                let func_index = self.allocate_and_copy_to_heap(serialized_func_info)?;
                // 関数の情報をグローバルコンテキストに保存
                self.context.global_context.insert(
                    func_name.clone(),
                    Variable {
                        value: Value::Number(serde_json::Number::from(func_index)),
                        data_type: Value::String("Function".into()),
                        address: func_index,
                        is_mutable: false,
                        size: func_info_size,
                    },
                );

                info!("FunctionDeclaration: name = {:?}, args = {:?}, body = {:?}, return_type = {:?}", func_name, arg_addresses, body, return_type);
                Ok(Value::Null)
            }

            NodeValue::VariableDeclaration(
                var_name,
                data_type,
                value,
                is_local,
                is_mutable,
                is_reference,
            ) => {
                // ステートメントフラグのチェック
                if !node.is_statement() {
                    return Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Variable declaration must be a statement"
                    ));
                }
                info!("is_reference: {:?}", is_reference);
                let name = match var_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };

                let v_type;
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
                            node.line(),
                            node.column(),
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
                        Value::String(self.infer_type(&_value))
                    } else {
                        let v = match data_type.node_value() {
                            NodeValue::DataType(v_type) => match v_type.node_value() {
                                NodeValue::Variable(v) => v,
                                _ => String::new(),
                            },
                            _ => String::new(),
                        };
                        Value::String(v.into())
                    };

                    v_value = if let NodeValue::Empty = value.node_value() {
                        Value::Number(serde_json::Number::from(0))
                    } else {
                        let _value = self.execute_node(&value)?;
                        self.check_type(&_value, v_type.as_str().unwrap_or(""))?
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
                                        node.line(),
                                        node.column(),
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
                                // 変数以外の値の場合、新しいアドレスを割り当てる
                                let serialized_value =
                                    self.serialize_value(v_type.as_str().unwrap_or(""), &v_value);
                                self.allocate_and_copy_to_heap(serialized_value)?
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
                            size: 0,
                        },
                    );
                } else {
                    let serialized_value =
                        self.serialize_value(v_type.as_str().unwrap_or(""), &v_value);
                    address = self.allocate_and_copy_to_heap(serialized_value)?;

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
                            size: 0,
                        },
                    );
                }

                info!("VariableDeclaration: name = {:?}, data_type = {:?}, value = {:?}, address = {:?}", name, v_type, v_value, address);
                result = v_value.clone();
                let line = self.current_node.clone().unwrap().1.line();
                let column = self.current_node.clone().unwrap().1.column();
                self.context
                    .used_context
                    .insert(name.clone(), (line, column, false));
                Ok(v_value)
            }

            NodeValue::TypeDeclaration(_type_name, _type) => {
                let name = match _type_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };
                if self.context.type_context.contains_key(&name) {
                    return Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
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
            NodeValue::Int(number) => Ok(Value::Number((*number).into())),

            NodeValue::Float(number) => {
                let n = Number::from_f64(*number).unwrap();
                Ok(Value::Number(n.into()))
            }

            NodeValue::String(s) => Ok(Value::String(s.clone())),
            NodeValue::Bool(b) => Ok(Value::Bool(*b)),

            NodeValue::Variable(name) => {
                let line = self.current_node.clone().unwrap().1.line();
                let column = self.current_node.clone().unwrap().1.column();
                self.context
                    .used_context
                    .insert(name.clone(), (line, column, true));

                if let Some(var) = self.context.local_context.get(name) {
                    // ローカルスコープで変数を見つけた場合
                    let index = var.address; // アドレスを取得
                    let value_size =
                        self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

                    info!(
                        "Index: {}, Value size: {}, Heap size: {}",
                        index,
                        value_size,
                        self.memory_mgr.heap.len()
                    );

                    self.get_value_from_heap(&self.memory_mgr.heap, index, value_size)
                } else if let Some(var) = self.context.global_context.get(name) {
                    // グローバルスコープで変数を見つけた場合
                    let index = var.address; // アドレスを取得
                    let value_size =
                        self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

                    info!(
                        "Index: {}, Value size: {}, Heap size: {}",
                        index,
                        value_size,
                        self.memory_mgr.heap.len()
                    );

                    self.get_value_from_heap(&self.memory_mgr.heap, index, value_size)
                } else {
                    Ok(Value::Null)
                }
            }

            NodeValue::Add(lhs, rhs) => {
                let left = self.execute_node(&lhs)?;
                let right = self.execute_node(&rhs)?;
                match (left.clone(), right.clone()) {
                    (Value::Number(l), Value::Number(r)) => {
                        if l.is_i64() && r.is_i64() {
                            let result = l.as_i64().unwrap() + r.as_i64().unwrap();
                            info!("Add: {} + {}", l, r);
                            Ok(Value::Number(serde_json::Number::from(result)))
                        } else {
                            info!("Add: {} + {}", l, r);
                            Ok(Value::Number(
                                serde_json::Number::from_f64(
                                    l.as_f64().unwrap() + r.as_f64().unwrap(),
                                )
                                .unwrap(),
                            ))
                        }
                    }
                    (Value::String(l), Value::String(r)) => {
                        let result = l.clone() + &r.clone();
                        info!("Add: {} + {}", l, r);
                        Ok(Value::String(result))
                    }

                    _ => Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Addition operation failed: {:?} + {:?}",
                        left.clone(),
                        right.clone()
                    )),
                }
            }

            NodeValue::Sub(lhs, rhs) => {
                let left = self.execute_node(&lhs)?;
                let right = self.execute_node(&rhs)?;
                match (left.clone(), right.clone()) {
                    (Value::Number(l), Value::Number(r)) => {
                        if l.is_i64() && r.is_i64() {
                            let result = l.as_i64().unwrap() - r.as_i64().unwrap();
                            info!("Sub: {} - {}", l, r);
                            Ok(Value::Number(serde_json::Number::from(result)))
                        } else {
                            info!("Sub: {} - {}", l, r);
                            Ok(Value::Number(
                                serde_json::Number::from_f64(
                                    l.as_f64().unwrap() - r.as_f64().unwrap(),
                                )
                                .unwrap(),
                            ))
                        }
                    }
                    _ => Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Subtraction operation failed: {:?} - {:?}",
                        left.clone(),
                        right.clone()
                    )),
                }
            }

            NodeValue::Mul(lhs, rhs) => {
                let left = self.execute_node(&lhs)?;
                let right = self.execute_node(&rhs)?;
                match (left.clone(), right.clone()) {
                    (Value::Number(l), Value::Number(r)) => {
                        if l.is_i64() && r.is_i64() {
                            let result = l.as_i64().unwrap() * r.as_i64().unwrap();
                            info!("Mul: {} * {}", l, r);
                            Ok(Value::Number(serde_json::Number::from(result)))
                        } else {
                            info!("Mul: {} * {}", l, r);
                            Ok(Value::Number(
                                serde_json::Number::from_f64(
                                    l.as_f64().unwrap() * r.as_f64().unwrap(),
                                )
                                .unwrap(),
                            ))
                        }
                    }
                    _ => Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Multiplication operation failed: {:?} * {:?}",
                        left.clone(),
                        right.clone()
                    )),
                }
            }

            NodeValue::Div(lhs, rhs) => {
                let left = self.execute_node(&lhs)?;
                let right = self.execute_node(&rhs)?;
                if let Value::Number(ref r) = right.clone() {
                    if r.as_f64().unwrap() == 0.0 {
                        return Err(compile_error!(
                            "error",
                            node.line(),
                            node.column(),
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Division by zero: {:?} / {:?}",
                            left.clone(),
                            right.clone()
                        ));
                    }
                }
                match (left.clone(), right.clone()) {
                    (Value::Number(l), Value::Number(r)) => {
                        if l.is_i64() && r.is_i64() {
                            let result = l.as_i64().unwrap() / r.as_i64().unwrap();
                            info!("Div: {} / {}", l, r);
                            Ok(Value::Number(serde_json::Number::from(result)))
                        } else {
                            info!("Div: {} / {}", l, r);
                            Ok(Value::Number(
                                serde_json::Number::from_f64(
                                    l.as_f64().unwrap() / r.as_f64().unwrap(),
                                )
                                .unwrap(),
                            ))
                        }
                    }
                    _ => Err(compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Division operation failed: {:?} / {:?}",
                        left.clone(),
                        right.clone()
                    )),
                }
            }

            NodeValue::Return(ret) => {
                let ret = self.execute_node(&ret)?;
                info!("Return: {:?}", ret);
                Ok(ret)
            }

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
