use crate::custom_compile_error;
use crate::lexer::{Lexer, Token};
use crate::parser::Node;
use crate::parser::Parser;
use crate::types::NodeValue;
use anyhow::{Context, Result as R};
use log::{error, info};
use property_rs::Property;
use serde_json::{Number, Value};
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::zip;
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
pub struct Decoder {
    // 現在のノード(ファイルパス,ノード)
    #[property(get)]
    current_node: Option<(String, Box<Node>)>,
    // ノードのマップ(ファイルパス,全体のノード)
    #[property(get)]
    nodes_map: HashMap<String, Vec<Box<Node>>>,
    // ノードのキャッシュ(ファイルパス,全体のノード)
    #[property(get)]
    cache: HashMap<String, Vec<Box<Node>>>,
    // グローバルスコープ(変数名,値)
    #[property(get)]
    global_context: HashMap<String, Variable>,
    // グローバル型定義(型名,型)
    #[property(get)]
    type_context: HashMap<String, String>,
    // コメントリスト((行数,列数),コメントのベクター)
    #[property(get)]
    comment_lists: HashMap<(usize, usize), Vec<String>>,
    // 最初に渡されたファイル(ファイルパス,内容)
    #[property(get)]
    first_file: (String, String),
    // 仮想ヒープ領域(u8のベクター)
    #[property(get)]
    heap: Vec<u8>,
    // フリーリスト(usizeのベクター)
    #[property(get)]
    free_list: Vec<usize>,
    #[property(get)]
    processed_files: HashSet<String>, // 追加
}

impl Decoder {
    pub fn new(file_path: String, content: String) -> Self {
        Self {
            cache: HashMap::new(),
            nodes_map: HashMap::new(),
            current_node: None,
            global_context: HashMap::new(),
            type_context: HashMap::new(),
            comment_lists: HashMap::new(),
            heap: vec![0; 1024 * 1024],
            free_list: Vec::new(),
            first_file: (file_path, content),
            processed_files: HashSet::new(),
        }
    }

    fn allocate(&mut self, size: usize) -> Result<usize, String> {
        if let Some(index) = self.free_list.pop() {
            Ok(index)
        } else {
            let index = self.heap.len();
            // ヒープのサイズが不足している場合、拡張する
            if index + size > self.heap.len() {
                let new_capacity = (index + size).max(self.heap.len() * 2);
                self.heap.resize(new_capacity, 0);
            }
            self.heap.resize(index + size, 0);
            Ok(index)
        }
    }

    fn deallocate(&mut self, index: usize) {
        self.free_list.push(index);
    }

    fn include_file(&mut self, filename: &str) -> Result<Vec<Box<Node>>, String> {
        if let Some(cached_nodes) = self.cache.get(filename) {
            return Ok(cached_nodes.clone());
        }
        let content = std::fs::read_to_string(filename).map_err(|e| e.to_string())?;
        let mut lexer = Lexer::new_with_value(filename.to_string(), content.clone());
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(&tokens, filename.to_string(), content);
        let nodes = parser.parse()?;
        self.cache.insert(filename.to_string(), nodes.clone());
        Ok(nodes)
    }

    pub fn add_include(
        &mut self,
        filepath: &str,
        nodes: &mut Vec<Box<Node>>,
    ) -> Result<(), String> {
        self.current_node = Some((filepath.to_string(), Box::new(Node::default())));

        let included_nodes = self.include_file(filepath)?;
        nodes.splice(0..0, included_nodes.clone());
        self.nodes_map.insert(filepath.to_string(), included_nodes);
        Ok(())
    }

    pub fn decode(&mut self, nodes: &mut Vec<Box<Node>>) -> Result<Value, String> {
        let mut result = Value::Null;
        let original_node = self.current_node.clone();
        self.add_include("./script/std.script", nodes)?;
        self.add_include(&self.first_file.0.clone(), nodes)?;

        for (filename, included_nodes) in self.nodes_map.clone() {
            self.current_node = Some((filename.clone(), Box::new(Node::default())));
            for node in included_nodes {
                self.current_node = Some((filename.clone(), node.clone()));
                match node.node_value() {
                    NodeValue::Include(filename) => {
                        self.current_node = Some((filename.clone(), node.clone()));
                        if !self.nodes_map.contains_key(&filename) {
                            self.add_include(&filename, nodes)?;
                        }
                        let mut included_nodes = self
                            .nodes_map
                            .get(&filename)
                            .ok_or("File not found")?
                            .clone();
                        result = self.decode(&mut included_nodes)?;
                        self.current_node = Some((filename.clone(), node.clone()));
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
    fn allocate_and_copy_to_heap(&mut self, serialized_value: Vec<u8>) -> Result<usize, String> {
        let node = if let Some((_, node)) = self.current_node.clone() {
            node
        } else {
            Box::new(Node::new(NodeValue::Empty, None, 0, 0))
        };
        let index = self.allocate(serialized_value.len())?;
        if index + serialized_value.len() > self.heap.len() {
            return Err(custom_compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &self.first_file.0,
                &self.first_file.1,
                "Heap overflow: trying to write {} bytes at index {}, but heap size is {}",
                serialized_value.len(),
                index,
                self.heap.len()
            ));
        }
        self.heap[index..index + serialized_value.len()].copy_from_slice(&serialized_value);
        Ok(index)
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
        let node = if let Some((_, node)) = self.current_node.clone() {
            node
        } else {
            Box::new(Node::new(NodeValue::Empty, None, 0, 0))
        };

        if index + value_size <= heap.len() {
            let v_value: Value = serde_json::from_slice(&heap[index..index + value_size]).unwrap();
            Ok(v_value)
        } else {
            return Err(custom_compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &self.first_file.0,
                &self.first_file.1,
                "Index out of range: {} + {} > {}",
                index,
                value_size,
                heap.len()
            ));
        }
    }

    fn infer_type(&self, value: &Value) -> String {
        match value {
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

    fn check_type(&self, value: &Value, expected_type: &str) -> Result<Value, String> {
        let node = if let Some((_, node)) = self.current_node.clone() {
            node
        } else {
            Box::new(Node::new(NodeValue::Empty, None, 0, 0))
        };

        // 型定義が存在するか確認
        if !self.type_context.contains_key(expected_type) {
            return Err(custom_compile_error!(
                "error",
                node.clone().line(),
                node.clone().column(),
                &self.first_file.0,
                &self.first_file.1,
                "Type '{}' is not defined",
                expected_type
            ));
        }

        match expected_type {
            "i32" => {
                if let Some(num) = value.as_i64() {
                    match i32::try_from(num) {
                        Ok(num_i32) => Ok(Value::Number(serde_json::Number::from(num_i32))),
                        Err(_) => Err(custom_compile_error!(
                            "error",
                            node.clone().line(),
                            node.clone().column(),
                            &self.first_file.0,
                            &self.first_file.1,
                            "Value out of range for i32: {:?}",
                            num
                        )),
                    }
                } else {
                    Err(custom_compile_error!(
                        "error",
                        node.clone().line(),
                        node.clone().column(),
                        &self.first_file.0,
                        &self.first_file.1,
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
                        &self.first_file.0,
                        &self.first_file.1,
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
                        &self.first_file.0,
                        &self.first_file.1,
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
                        &self.first_file.0,
                        &self.first_file.1,
                        "Type mismatch for f64: {:?}",
                        value
                    ))
                }
            }
            _ => Ok(value.clone()),
        }
    }
    fn execute_node(&mut self, node: &Node) -> Result<Value, String> {
        let mut result = Value::Null;
        info!("current_node: {:?}", self.current_node());
        match &node.node_value() {
            NodeValue::Empty | NodeValue::StatementEnd => Ok(result),
            NodeValue::MultiComment(content, (line, column)) => {
                self.comment_lists
                    .insert((*line, *column), content.clone().to_vec());
                info!("MultiComment added at line {}, column {}", line, column);
                Ok(result)
            }
            NodeValue::SingleComment(content, (line, column)) => {
                self.comment_lists
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
            NodeValue::VariableDeclaration(var_name, data_type, value) => {
                let name = match var_name.node_value() {
                    NodeValue::Variable(v) => v,
                    _ => String::new(),
                };

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

                self.global_context.insert(
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
                self.type_context.insert(name.clone(), v_type.clone());

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
                if let Some(var) = self.global_context.get(name) {
                    let index = var.address; // アドレスを取得
                    let value_size =
                        self.get_value_size(var.data_type.as_str().unwrap_or(""), &var.value);

                    println!(
                        "Index: {}, Value size: {}, Heap size: {}",
                        index,
                        value_size,
                        self.heap.len()
                    );

                    self.get_value_from_heap(&self.heap, index, value_size)
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
                    _ => Err(custom_compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.first_file.0,
                        &self.first_file.1,
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
                        &self.first_file.0,
                        &self.first_file.1,
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
                            &self.first_file.0,
                            &self.first_file.1,
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
                        &self.first_file.0,
                        &self.first_file.1,
                        "Division operation failed: {:?} / {:?}",
                        left.clone(),
                        right.clone()
                    )),
                }
            }

            NodeValue::Function(name, params, body, _, _) => {
                self.global_context.insert(
                    name.clone(),
                    Variable {
                        data_type: Value::String("function".to_string()),
                        value: Value::Null,
                        address: 0,
                    },
                );
                info!("defined Function: {}", name);
                self.execute_node(&body)
            }
            NodeValue::Call(name, args) => {
                if let Some(Variable {
                    data_type: _,
                    value: Value::Null,
                    address: 0,
                }) = self.global_context.get(name)
                {
                    for (i, arg) in args.iter().enumerate() {
                        self.global_context.clone().insert(
                            format!("arg{}", i),
                            Variable {
                                data_type: Value::String("argument".to_string()),
                                value: self.execute_node(arg)?,
                                address: 0,
                            },
                        );
                    }
                    info!("Call: {}", name);
                    self.execute_node(&Node::new(NodeValue::Block(vec![]), None, 0, 0))
                } else {
                    Err(custom_compile_error!(
                        "error",
                        node.line(),
                        node.column(),
                        &self.first_file.0,
                        &self.first_file.1,
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
                &self.first_file.0,
                &self.first_file.1,
                "Unknown node value: {:?}",
                node.node_value()
            )),
        }
    }
}
