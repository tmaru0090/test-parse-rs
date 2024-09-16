use crate::custom_compile_error;
use crate::lexer::{Lexer, Token};
use crate::parser::Node;
use crate::parser::Parser;
use crate::types::NodeValue;
use anyhow::Result as R;
use indexmap::IndexMap;
use log::info;
use property_rs::Property;
use serde_json::{Number, Value};
use std::collections::HashSet;
use std::ops::{Add, Div, Mul, Sub};

impl Add for Variable {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let result = match (self.value, other.value) {
            (Value::Number(a), Value::Number(b)) => {
                Value::Number((a.as_i64().unwrap() + b.as_i64().unwrap()).into()).into()
            }
            (Value::String(a), Value::String(b)) => Value::String(a + &b).into(),
            _ => panic!("Unsupported types for addition"),
        };
        Variable {
            data_type: self.data_type,
            value: result,
            address: 0,
        }
    }
}

impl Sub for Variable {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let result = match (self.value, other.value) {
            (Value::Number(a), Value::Number(b)) => {
                Value::Number((a.as_i64().unwrap() - b.as_i64().unwrap()).into()).into()
            }
            _ => panic!("Unsupported types for subtraction"),
        };
        Variable {
            data_type: self.data_type,
            value: result,
            address: 0,
        }
    }
}

impl Mul for Variable {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let result = match (self.value, other.value) {
            (Value::Number(a), Value::Number(b)) => {
                Value::Number((a.as_i64().unwrap() * b.as_i64().unwrap()).into()).into()
            }
            _ => panic!("Unsupported types for multiplication"),
        };
        Variable {
            data_type: self.data_type,
            value: result,
            address: 0,
        }
    }
}

impl Div for Variable {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let result = match (self.value, other.value) {
            (Value::Number(a), Value::Number(b)) => {
                Value::Number((a.as_i64().unwrap() / b.as_i64().unwrap()).into()).into()
            }
            _ => panic!("Unsupported types for division"),
        };
        Variable {
            data_type: self.data_type,
            value: result,
            address: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    data_type: Value,
    value: Value,
    address: usize,
}
#[derive(Debug, Clone, Property)]
pub struct Context {
    pub global_context: IndexMap<String, Variable>,
    pub type_context: IndexMap<String, String>,
    pub comment_lists: IndexMap<(usize, usize), Vec<String>>,
}
impl Context {
    fn new() -> Self {
        Context {
            global_context: IndexMap::new(),
            type_context: IndexMap::new(),
            comment_lists: IndexMap::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct FileCache {
    pub cache: IndexMap<String, Vec<Box<Node>>>,
    pub processed_files: HashSet<String>,
}
impl FileCache {
    fn new() -> Self {
        FileCache {
            processed_files: HashSet::new(),
            cache: IndexMap::new(),
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
    current_node: Option<(String, Box<Node>)>,
    #[property(get)]
    nodes_map: IndexMap<String, Vec<Box<Node>>>,
    #[property(get)]
    memory_mgr: MemoryManager,
    #[property(get)]
    context: Context,
    #[property(get)]
    file_contents: IndexMap<String, String>,
    #[property(get)]
    file_cache: FileCache,
    #[property(get)]
    first_file: (String, String),
}

impl Decoder {
    pub fn new(file_path: String, content: String) -> Self {
        Self {
            memory_mgr: MemoryManager::new(1024 * 1024),
            nodes_map: IndexMap::new(),
            file_contents: IndexMap::new(),
            current_node: None,
            first_file: (file_path, content),
            context: Context::new(),
            file_cache: FileCache::new(),
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
    fn include_file(&mut self, filename: &str) -> R<Vec<Box<Node>>, String> {
        if self.file_cache.processed_files.contains(filename) {
            return Ok(vec![]); // 既に処理済みの場合は空のノードを返す
        }
        if let Some(cached_nodes) = self.file_cache.cache.get(filename) {
            return Ok(cached_nodes.clone());
        }
        let content = std::fs::read_to_string(filename).map_err(|e| e.to_string())?;
        let mut lexer = Lexer::new_with_value(filename.to_string(), content.clone());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(&tokens, filename.to_string(), content.clone());
        let nodes = parser.parse()?;
        self.file_cache
            .cache
            .insert(filename.to_string(), nodes.clone());
        self.file_cache.processed_files.insert(filename.to_string()); // 処理済みファイルとしてマーク
        self.file_contents.insert(filename.to_string(), content); // ファイル内容を保存
        Ok(nodes)
    }
    pub fn add_include(&mut self, filename: &str, nodes: &mut Vec<Box<Node>>) -> R<(), String> {
        let included_nodes = self.include_file(filename)?;
        nodes.splice(0..0, included_nodes.clone());
        self.nodes_map.insert(filename.to_string(), included_nodes);
        Ok(())
    }

    pub fn decode(&mut self, nodes: &mut Vec<Box<Node>>) -> R<Value, String> {
        let mut result = Value::Null;
        let original_node = self.current_node.clone();

        // std.scriptを読み込む
        self.add_include("./script/std.script", nodes)?;
        // 他のファイルを読み込む
        self.add_include(&self.first_file.0.clone(), nodes)?;
        for (filename, included_nodes) in self.nodes_map.clone() {
            self.current_node = Some((filename.clone(), Box::new(Node::default())));
            for node in included_nodes {
                self.current_node = Some((filename.clone(), node.clone()));
                match node.node_value() {
                    NodeValue::Include(filename) => {
                        if !self.nodes_map.contains_key(&filename) {
                            self.add_include(&filename, nodes)?;
                        }
                        let mut included_nodes = self
                            .nodes_map
                            .get(&filename)
                            .ok_or("File not found")?
                            .clone();
                        result = self.decode(&mut included_nodes)?;
                    }
                    _ => {
                        result = self.execute_node(&node)?;
                    }
                }
            }
        }

        self.current_node = original_node;
        Ok(result)
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
            return Err(custom_compile_error!(
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
            return Err(custom_compile_error!(
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
            Err(custom_compile_error!(
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
            Value::Null => "unit".to_string(),
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
            return Err(custom_compile_error!(
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
                        Err(_) => Err(custom_compile_error!(
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
                    Err(custom_compile_error!(
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
                    Err(custom_compile_error!(
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
                    Err(custom_compile_error!(
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
                    Err(custom_compile_error!(
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
    fn execute_node(&mut self, node: &Node) -> R<Value, String> {
        let mut result = Value::Null;

        info!("global_contexts: {:?}", self.context.global_context.clone());
        match &node.node_value() {
            NodeValue::Empty | NodeValue::StatementEnd => Ok(result),
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
                for b in block {
                    r = self.execute_node(b)?;
                }
                Ok(r)
            }
            NodeValue::Assign(var_name, value) => {
                let name = match var_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };

                // 変数のデータを一時変数にコピー
                let variable_data = self.context.global_context.get(&name).cloned();

                if let Some(mut variable) = variable_data {
                    let new_value = self.execute_node(&value)?;
                    self.check_type(&new_value, variable.data_type.as_str().unwrap_or(""))?;

                    let serialized_value =
                        self.serialize_value(variable.data_type.as_str().unwrap_or(""), &new_value);
                    self.copy_to_heap(variable.address, serialized_value)?;

                    // 変数の値を更新
                    variable.value = new_value.clone();
                    self.context.global_context.insert(name.clone(), variable);
                    info!("Assign: name = {:?}, new_value = {:?}", name, new_value);
                    result = new_value.clone();

                    Ok(new_value)
                } else {
                    Err(custom_compile_error!(
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
            NodeValue::VariableDeclaration(var_name, data_type, value) => {
                let name = match var_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };

                if self.context.global_context.contains_key(&name) {
                    return Err(custom_compile_error!(
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
                } else {
                    let v_type = if let NodeValue::Empty = data_type.node_value() {
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

                    let v_value = if let NodeValue::Empty = value.node_value() {
                        Value::Number(serde_json::Number::from(0))
                    } else {
                        let _value = self.execute_node(&value)?;
                        self.check_type(&_value, v_type.as_str().unwrap_or(""))?
                    };

                    let serialized_value =
                        self.serialize_value(v_type.as_str().unwrap_or(""), &v_value);
                    let index = self.allocate_and_copy_to_heap(serialized_value)?;

                    self.context.global_context.insert(
                        name.clone(),
                        Variable {
                            value: v_value.clone(),
                            data_type: v_type.clone(),
                            address: index,
                        },
                    );
                    info!("VariableDeclaration: name = {:?}, data_type = {:?}, value = {:?}, address = {:?}", name, v_type, v_value, index);
                    result = v_value.clone();

                    Ok(v_value)
                }
            }

            NodeValue::TypeDeclaration(_type_name, _type) => {
                let name = match _type_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };
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
                if let Some(var) = self.context.global_context.get(name) {
                    let index = var.address; // アドレスを取得
                    let value_size =
                        self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

                    println!(
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

                    _ => Err(custom_compile_error!(
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
                    _ => Err(custom_compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.first_file.0,
                        &self.first_file.1,
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
                    _ => Err(custom_compile_error!(
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
                        return Err(custom_compile_error!(
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
                    _ => Err(custom_compile_error!(
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
            NodeValue::Function(name, params, body, return_value, return_type) => {
                // 関数名をヒープに保存
                let serialized_name = self.serialize_value("string", &Value::String(name.clone()));
                let name_index = self.allocate_and_copy_to_heap(serialized_name)?;

                // 引数名リストをヒープに保存
                let serialized_params = self.serialize_value(
                    "params",
                    &Value::Array(params.iter().map(|p| Value::String(p.clone())).collect()),
                );
                let params_index = self.allocate_and_copy_to_heap(serialized_params)?;

                // 関数本体をヒープに保存
                let serialized_body =
                    self.serialize_value("body", &Value::String(format!("{:?}", body)));
                let body_index = self.allocate_and_copy_to_heap(serialized_body)?;

                // 戻り値をヒープに保存
                let serialized_return_value = self.serialize_value(
                    "return_value",
                    &Value::String(format!("{:?}", return_value)),
                );
                let return_value_index = self.allocate_and_copy_to_heap(serialized_return_value)?;

                // 戻り値の型をヒープに保存
                let serialized_return_type = self
                    .serialize_value("return_type", &Value::String(format!("{:?}", return_type)));
                let return_type_index = self.allocate_and_copy_to_heap(serialized_return_type)?;

                // グローバルコンテキストに関数を登録
                self.context.global_context.insert(
                    name.clone(),
                    Variable {
                        data_type: Value::String("function".to_string()),
                        value: Value::Null,
                        address: name_index, // 関数名のアドレスを保存
                    },
                );

                // ログ出力
                info!(
        "FunctionDefinition: name = {:?}, params = {:?}, body = {:?}, return_value = {:?}, return_type = {:?}, name_address = {:?}, params_address = {:?}, body_address = {:?}, return_value_address = {:?}, return_type_address = {:?}",
        name, params, body, return_value, return_type, name_index, params_index, body_index, return_value_index, return_type_index
    );
                self.execute_node(&body)
            }

            NodeValue::Call(name, args) => {
                if let Some(Variable {
                    data_type: _,
                    value: Value::Null,
                    address: 0,
                }) = self.context.global_context.get(name)
                {
                    for (i, arg) in args.iter().enumerate() {
                        let arg_value = self.execute_node(arg)?;
                        let serialized_arg = self.serialize_value("argument", &arg_value);
                        let arg_index = self.allocate_and_copy_to_heap(serialized_arg)?;

                        self.context.global_context.insert(
                            format!("arg{}", i),
                            Variable {
                                data_type: Value::String("argument".to_string()),
                                value: arg_value.clone(),
                                address: arg_index,
                            },
                        );

                        info!(
                            "Argument: index = {}, value = {:?}, address = {:?}",
                            i, arg_value, arg_index
                        );
                    }

                    info!("Call: {}", name);
                    self.execute_node(&Node::new(NodeValue::Block(vec![]), None, 0, 0))
                } else {
                    Err(custom_compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Function call failed: {:?}",
                        name
                    ))
                }
            }
            NodeValue::Return(ret) => {
                let ret = self.execute_node(&ret)?;
                info!("Return: {:?}", ret);
                Ok(ret)
            }
            _ => Err(custom_compile_error!(
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
