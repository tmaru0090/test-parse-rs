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

/*
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
*/
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
impl Clone for MemoryBlock {
    fn clone(&self) -> Self {
        // クローン処理。今回はidのみクローンし、valueはクローン不可のため新たに初期化
        MemoryBlock {
            id: self.id,
            value: Box::new(()), // クローンできないためデフォルトの空の値を持たせる
        }
    }
}

#[derive(Debug, Clone)]
struct MemoryManager {
    pub heap: HashMap<Uuid, MemoryBlock>,
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

#[derive(Debug, Clone)]
pub struct Variable {
    data_type: Value,
    value: Value,
    address: Uuid,
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

    #[property(get)]
    entry_func: (bool, String),
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
    pub fn add_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        self.ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(self)
    }

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

    /*
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
    */
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

    fn deallocate(&mut self, id: Uuid) {
        if self.memory_mgr.heap.remove(&id).is_some() {
            self.memory_mgr.free_list.push(id); // 解放されたメモリブロックをフリーリストに追加
        }
    }

    fn get_value<T: 'static + Any>(&self, id: Uuid) -> Option<&T> {
        self.memory_mgr
            .heap
            .get(&id)
            .and_then(|block| block.value.downcast_ref::<T>()) // IDから値を取得
    }
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
        for (file_name, nodes) in self.ast_map() {
            self.current_node = Some((file_name.clone(), Box::new(Node::default())));
            let content = std::fs::read_to_string(file_name.clone()).map_err(|e| e.to_string())?;
            self.file_contents.insert(file_name.clone(), content);
            for node in nodes {
                self.current_node = Some((file_name.clone(), node.clone()));
                value = self.execute_node(&node)?;
            }
        }

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

    fn execute_node(&mut self, node: &Node) -> R<Value, String> {
        let mut result = Value::Null;
        //info!("global_contexts: {:?}", self.context.global_context.clone());
        //info!("local_contexts: {:?}", self.context.local_context.clone());
        //info!("used_context: {:?}", self.context.used_context.clone());
        //info!("current_node: {:?}", self.current_node.clone());
        let node_value = node.clone().node_value();
        match &node_value {
            NodeValue::Include(file_name) => {
                self.add_first_ast_from_file(file_name)?;
                let ast_map = self.ast_map();
                let nodes = ast_map.get(file_name).unwrap();
                for node in nodes {
                    self.execute_node(&node)?;
                }
                Ok(Value::Null)
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

            NodeValue::Array(data_type, values) => {
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

            NodeValue::Block(block) => {
                let mut r = Value::Null;
                let initial_local_context = self.context.local_context.clone(); // 現在のローカルコンテキストを保存
                for b in block {
                    info!("local_context: {:?}", self.context.local_context.clone());
                    r = self.execute_node(b)?;
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
                let func_name = name;
                let variables = {
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

                let mut evaluated_args = Vec::new();
                for arg in args {
                    let evaluated_arg = self.execute_node(&arg)?;
                    evaluated_args.push(evaluated_arg);
                }

                // func_infoから関数のアドレスを取得
                let func_address = variables.address;
                let func_info = self.get_value::<Value>(func_address).unwrap();
                let _args = func_info["args"].clone();
                let _body = func_info["body"].clone();
                let body: Node = serde_json::from_value(_body).unwrap();
                let return_type = func_info["return_type"].clone();

                for arg in _args.as_array().unwrap() {
                    let _arg_name = arg["name"].clone();
                    let arg_name = _arg_name.as_str().unwrap();
                    let arg_type = arg["type"].clone();
                    for value in &evaluated_args {
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
                }
                result = self.execute_node(&body)?;
                info!(
                    "CallFunction: name = {:?},args = {:?},return_value = {:?}",
                    func_name,
                    evaluated_args.clone(),
                    result
                );

                Ok(result)
            }

            NodeValue::CallBackFunction(name, args, body, return_type, is_system) => {
                Ok(Value::Null)
            }

            NodeValue::Function(name, args, body, return_type, is_system) => {
                let func_name = name; // すでに String 型なのでそのまま使う
                if func_name == "main" || func_name == "Main" {
                    self.entry_func.0 = true;
                    self.entry_func.1 = func_name.clone();
                }
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

                let mut arg_addresses = Vec::new();

                let func_index = self.allocate(func_name.clone());

                for (i, (data_type, arg_name)) in args.iter().enumerate() {
                    arg_addresses
                        .push(serde_json::json!({"name": arg_name.clone(),"type": data_type}));
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
                            size: 0,
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
                            size: 0,
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

                    let value = self.get_value::<Value>(index).unwrap();
                    Ok(value.clone())
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
                    let value = self.get_value::<Value>(index).unwrap();
                    Ok(value.clone())
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
