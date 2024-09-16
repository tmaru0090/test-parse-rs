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
            is_mutable: false,
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
            is_mutable: false,
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
            is_mutable: false,
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
            is_mutable: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    data_type: Value,
    value: Value,
    address: usize,
    is_mutable: bool, // 可変性を示すフィールドを追加
}

#[derive(Debug, Clone, Property)]
pub struct Context {
    pub local_context: IndexMap<String, Variable>,
    pub global_context: IndexMap<String, Variable>,
    pub type_context: IndexMap<String, String>,
    pub comment_lists: IndexMap<(usize, usize), Vec<String>>,
}
impl Context {
    fn new() -> Self {
        Context {
            local_context: IndexMap::new(),
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
        info!("local_contexts: {:?}", self.context.local_context.clone());
        match &node.node_value() {
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
                        };
                        let value_name = match value.node_value() {
                            NodeValue::Variable(v) => v,
                            _ => {
                                return Err(custom_compile_error!(
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
                    Err(custom_compile_error!(
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
                    let temp_string = String::new();
                    /*
                    info!("type: {:?}",data_type.clone());
                    let v_type = match v_type {
                        Value::String(ref v) => v,
                        _ => &temp_string,
                    };
                    self.check_type(&v_value, &v_type)?;
                    */
                    evaluated_values.push(v_value);
                }
                // 配列をシリアライズしてヒープにコピー
                let serialized_array =
                    self.serialize_value("array", &Value::Array(evaluated_values.clone()));
                let index = self.allocate_and_copy_to_heap(serialized_array.clone())?;
                info!(
                    "serialized_array: {:?} index: {:?}",
                    serialized_array.clone(),
                    index
                );
                // 結果を返す
                Ok(Value::Array(evaluated_values))
            }
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
                let initial_local_context = self.context.local_context.clone(); // 現在のローカルコンテキストを保存

                for b in block {
                    r = self.execute_node(b)?;
                }

                self.context.local_context = initial_local_context; // ブロックの処理が終わったらローカルコンテキストを元に戻す
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
                            "Variable '{}' is not mutable",
                            name
                        ))
                    }
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

            /*
                        NodeValue::Call(func_name, args) => {
                            // 関数が定義されているかチェック
                            let func_var = match self.context.global_context.get(func_name) {
                                Some(f) => f,
                                None => {
                                    return Err(custom_compile_error!(
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
                                    ));
                                }
                            };

                            // ヒープから関数の情報を取得
                            let func_index = match func_var.value {
                                Value::Number(ref n) => n.as_u64().unwrap() as usize,
                                _ => {
                                    return Err(custom_compile_error!(
                                        "error",
                                        node.line(),
                                        node.column(),
                                        &self.current_node.clone().unwrap().0,
                                        &self
                                            .file_contents
                                            .get(&self.current_node.clone().unwrap().0)
                                            .unwrap(),
                                        "Invalid function index"
                                    ))
                                }
                            };
                            let serialized_func_info = self
                                .read_from_heap(func_index, self.get_value_size("Function", &func_var.value))?;
                            let func_info: serde_json::Value =
                                serde_json::from_slice(&serialized_func_info).map_err(|e| e.to_string())?;

                            // 引数を設定
                            let func_args = func_info["args"].as_array().unwrap();
                            let body = serde_json::from_value::<Node>(func_info["body"].clone())
                                .map_err(|e| e.to_string())?;
                            let return_type = &func_info["return_type"];

                            for (i, arg) in args.iter().enumerate() {
                                let arg_value = self.execute_node(arg)?;
                                let arg_name = func_args[i]["0"].as_str().unwrap();
                                let arg_address = func_args[i]["1"].as_u64().unwrap() as usize;
                                self.context.local_context.insert(
                                    arg_name.to_string(),
                                    Variable {
                                        value: arg_value.clone(),
                                        data_type: Value::String("Argument".into()),
                                        address: arg_address,
                                    },
                                );
                            }

                            // 関数のボディを実行
                            let result = self.execute_node(&body)?;

                            // ローカルコンテキストをクリア
                            self.context.local_context.clear();

                            // 戻り値の型チェック
                            self.check_type(&result, return_type.as_str().unwrap_or(""))?;
                            Ok(result)
                        }
            */
            NodeValue::Function(name, args, body, return_type, _) => {
                let func_name = name; // すでに String 型なのでそのまま使う

                // 関数がすでに定義されているかチェック
                if self.context.global_context.contains_key(func_name.as_str()) {
                    return Err(custom_compile_error!(
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
                for (i, arg) in args.iter().enumerate() {
                    let arg_name = arg; // すでに String 型なのでそのまま使う

                    let serialized_value =
                        serde_json::to_vec(&Value::Number(serde_json::Number::from(i)))
                            .map_err(|e| e.to_string())?;
                    let index = self.allocate_and_copy_to_heap(serialized_value)?;
                    arg_addresses.push((arg_name.clone(), index));
                }

                // 関数の情報をシリアライズしてヒープに格納
                let func_info = serde_json::json!({
                    "args": arg_addresses,
                    "body": body,
                    "return_type": return_type,
                });
                let serialized_func_info =
                    serde_json::to_vec(&func_info).map_err(|e| e.to_string())?;
                let func_index = self.allocate_and_copy_to_heap(serialized_func_info)?;

                // 関数の情報をグローバルコンテキストに保存
                self.context.global_context.insert(
                    func_name.clone(),
                    Variable {
                        value: Value::Number(serde_json::Number::from(func_index)),
                        data_type: Value::String("Function".into()),
                        address: func_index,
                        is_mutable: false,
                    },
                );

                info!("FunctionDeclaration: name = {:?}, args = {:?}, body = {:?}, return_type = {:?}", func_name, arg_addresses, body, return_type);
                Ok(Value::Null)
            }
            NodeValue::VariableDeclaration(var_name, data_type, value, is_local, is_mutable) => {
                let name = match var_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };

                let v_type;
                let v_value;
                {
                    // 一時的にcontextの借用を解除
                    let context = if *is_local {
                        &mut self.context.local_context
                    } else {
                        &mut self.context.global_context
                    };

                    if context.contains_key(&name) {
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

                let serialized_value =
                    self.serialize_value(v_type.as_str().unwrap_or(""), &v_value);
                let index = self.allocate_and_copy_to_heap(serialized_value)?;

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
                        address: index,
                        is_mutable: *is_mutable,
                    },
                );

                info!("VariableDeclaration: name = {:?}, data_type = {:?}, value = {:?}, address = {:?}", name, v_type, v_value, index);
                result = v_value.clone();

                Ok(v_value)
            }

            NodeValue::TypeDeclaration(_type_name, _type) => {
                let name = match _type_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };
                if self.context.type_context.contains_key(&name) {
                    return Err(custom_compile_error!(
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
                if let Some(var) = self.context.local_context.get(name) {
                    // ローカルスコープで変数を見つけた場合
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
                } else if let Some(var) = self.context.global_context.get(name) {
                    // グローバルスコープで変数を見つけた場合
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
