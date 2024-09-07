
use crate::parser::Node;
use crate::types::NodeType;
use std::collections::HashMap;
use std::iter::zip;

use anyhow::Result as R;
use log::info;

pub struct Decoder {
    global_variables: HashMap<String, i32>,
    local_variables_stack: Vec<HashMap<String, i32>>,
    func_lists: HashMap<String, (Vec<String>, Box<Node>)>, // 関数の定義を保持
    last_var_name: Option<String>,                         // 最後に代入された変数名を保持
}

impl Decoder {
    pub fn evaluate(&mut self, node: &Box<Node>) -> R<i32, String> {
        match &node.node_value() {
            NodeType::Function(func_name, args, body) => {
                self.func_lists
                    .insert(func_name.clone(), (args.clone(), body.clone()));
                info!("define function: {:?}", func_name);
                Ok(0)
            }
            NodeType::Return(ret_value) => {
                let value = self.evaluate(ret_value)?;
                info!("Return: {:?}", value);
                Ok(value)
            }
            NodeType::Call(func_name, args) => {
                if let Some((args_name_v, body)) = self.func_lists.get(func_name).cloned() {
                    // 新しいローカル変数のスコープを作成し、引数をローカル変数として追加
                    let mut local_vars = HashMap::new();
                    for (arg_name, arg) in args_name_v.iter().zip(args.iter()) {
                        let arg_value = self.evaluate(&Box::new(arg.clone()))?;
                        local_vars.insert(arg_name.clone(), arg_value);
                    }
                    info!(
                        "Local variables for function {}: {:?}",
                        func_name, local_vars
                    );
                    self.local_variables_stack.push(local_vars);

                    // 関数本体を評価
                    let result = self.evaluate(&body);

                    // ローカル変数のスコープを削除
                    self.local_variables_stack.pop();

                    result
                } else {
                    Err(format!("Undefined function: {}", func_name))
                }
            }
            NodeType::Number(value) => Ok(*value),
            NodeType::Variable(name) => {
                // ローカル変数スタックのすべてのスコープを逆順でチェック
                for local_vars in self.local_variables_stack.iter().rev() {
                    if let Some(value) = local_vars.get(name) {
                        return Ok(*value);
                    }
                }
                // グローバル変数のチェック
                if let Some(value) = self.global_variables.get(name) {
                    Ok(*value)
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            NodeType::Assign(var_node, expr_node) => {
                let value = self.evaluate(expr_node)?;
                if let NodeType::Variable(var_name) = &var_node.node_value() {
                    if let Some(local_vars) = self.local_variables_stack.last_mut() {
                        local_vars.insert(var_name.clone(), value);
                    } else {
                        self.global_variables.insert(var_name.clone(), value);
                    }
                    self.last_var_name = Some(var_name.clone());
                    info!("Assigning: {} = {}", var_name, value);
                } else {
                    return Err("Left-hand side of assignment must be a variable.".to_string());
                }
                Ok(value)
            }
            NodeType::Block(nodes) => {
                // 新しいローカル変数のスコープを作成
                self.local_variables_stack.push(HashMap::new());
                let mut value: i32 = 0;
                for node in nodes {
                    let current_node: Box<Node> = Box::new(node.clone());
                    value = self.evaluate(&current_node)?;
                }
                // ブロックを抜けるときにローカル変数のスコープを削除
                self.local_variables_stack.pop();
                Ok(value)
            }
            NodeType::Add | NodeType::Sub | NodeType::Mul | NodeType::Div => {
                let current_node = node;

                let left_node = {
                    let temp_node = current_node.node_next();
                    temp_node
                        .as_ref()
                        .map(|n| n.clone())
                        .ok_or_else(|| "Missing left operand".to_string())?
                };
                let right_node = {
                    let temp_node = left_node.node_next();
                    temp_node
                        .as_ref()
                        .map(|n| n.clone())
                        .ok_or_else(|| "Missing right operand".to_string())?
                };

                let left_value = self.evaluate(&left_node)?;
                let right_value = self.evaluate(&right_node)?;
                match &node.node_value() {
                    NodeType::Add => Ok(left_value + right_value),
                    NodeType::Sub => Ok(left_value - right_value),
                    NodeType::Mul => Ok(left_value * right_value),
                    NodeType::Div => {
                        if right_value == 0 {
                            Err("Division by zero.".to_string())
                        } else {
                            Ok(left_value / right_value)
                        }
                    }
                    _ => Err("Unsupported operation".to_string()),
                }
            }
            _ => Err("Unsupported node type".to_string()),
        }
    }
    pub fn decode(&mut self, nodes: &Vec<Node>) -> R<(), String> {
        for node in nodes {
            match &node.node_value() {
                NodeType::Assign(var_node, expr_node) => {
                    let value = self.evaluate(expr_node)?;
                    if let NodeType::Variable(var_name) = &var_node.node_value() {
                        self.global_variables.insert(var_name.clone(), value);
                        info!("{} = {}", var_name, value);
                    } else {
                        return Err("Left-hand side of assignment must be a variable.".to_string());
                    }
                }
                _ => {
                    let value = self.evaluate(&Box::new(node.clone()))?;
                    //info!("Result: {}", value);
                }
            }
        }
        Ok(())
    }
}
impl Decoder {
    pub fn new() -> Self {
        Decoder {
            global_variables: HashMap::new(),
            local_variables_stack: Vec::new(),
            func_lists: HashMap::new(),
            last_var_name: None,
        }
    }
}







/*
use crate::parser::Node;
use crate::types::NodeType;
use std::collections::HashMap;
use std::iter::zip;

use anyhow::Result as R;
use log::info;

pub struct Decoder {
    global_variables: HashMap<String, i32>,
    local_variables_stack: Vec<HashMap<String, i32>>,
    func_lists: HashMap<String, (Vec<String>, Box<Node>)>, // 関数の定義を保持
    last_var_name: Option<String>,                         // 最後に代入された変数名を保持
}

impl Decoder {
    pub fn evaluate(&mut self, node: &Box<Node>) -> R<i32, String> {
        match &node.node_value() {
            NodeType::Function(func_name, args, body) => {
                self.func_lists
                    .insert(func_name.clone(), (args.clone(), body.clone()));
                info!("define function: {:?}", func_name);
                Ok(0)
            }
            NodeType::Return(ret_value) => {
                let value = self.evaluate(ret_value)?;
                info!("Return: {:?}", value);
                Ok(value)
            }
            NodeType::Call(func_name, args) => {
                if let Some((args_name_v, body)) = self.func_lists.get(func_name).cloned() {
                    // 新しいローカル変数のスコープを作成し、引数をローカル変数として追加
                    let mut local_vars = HashMap::new();
                    for (i, arg) in args.iter().enumerate() {
                        let arg_value = self.evaluate(&Box::new(arg.clone()))?;
                        for args_name in &args_name_v {
                            local_vars.insert(args_name.clone(), arg_value);
                        }
                    }
                    info!(
                        "Local variables for function {}: {:?}",
                        func_name, local_vars
                    );
                    self.local_variables_stack.push(local_vars);

                    // 関数本体を評価
                    let result = self.evaluate(&body);

                    // ローカル変数のスコープを削除
                    self.local_variables_stack.pop();

                    result
                } else {
                    Err(format!("Undefined function: {}", func_name))
                }
            }
            NodeType::Number(value) => Ok(*value),
            NodeType::Variable(name) => {
                // ローカル変数スタックのすべてのスコープを逆順でチェック
                for local_vars in self.local_variables_stack.iter().rev() {
                    if let Some(value) = local_vars.get(name) {
                        return Ok(*value);
                    }
                }
                // グローバル変数のチェック
                if let Some(value) = self.global_variables.get(name) {
                    Ok(*value)
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            NodeType::Assign(var_node, expr_node) => {
                let value = self.evaluate(expr_node)?;
                if let NodeType::Variable(var_name) = &var_node.node_value() {
                    if let Some(local_vars) = self.local_variables_stack.last_mut() {
                        local_vars.insert(var_name.clone(), value);
                    } else {
                        self.global_variables.insert(var_name.clone(), value);
                    }
                    self.last_var_name = Some(var_name.clone());
                    info!("Assigning: {} = {}", var_name, value);
                } else {
                    return Err("Left-hand side of assignment must be a variable.".to_string());
                }
                Ok(value)
            }
            NodeType::Block(nodes) => {
                // 新しいローカル変数のスコープを作成
                self.local_variables_stack.push(HashMap::new());
                let mut value: i32 = 0;
                for node in nodes {
                    let current_node: Box<Node> = Box::new(node.clone());
                    value = self.evaluate(&current_node)?;
                }
                // ブロックを抜けるときにローカル変数のスコープを削除
                self.local_variables_stack.pop();
                Ok(value)
            }
            NodeType::Add | NodeType::Sub | NodeType::Mul | NodeType::Div => {
                let current_node = node;

                let left_node = {
                    let temp_node = current_node.node_next();
                    temp_node
                        .as_ref()
                        .map(|n| n.clone())
                        .ok_or_else(|| "Missing left operand".to_string())?
                };
                let right_node = {
                    let temp_node = left_node.node_next();
                    temp_node
                        .as_ref()
                        .map(|n| n.clone())
                        .ok_or_else(|| "Missing right operand".to_string())?
                };

                let left_value = self.evaluate(&left_node)?;
                let right_value = self.evaluate(&right_node)?;
                match &node.node_value() {
                    NodeType::Add => Ok(left_value + right_value),
                    NodeType::Sub => Ok(left_value - right_value),
                    NodeType::Mul => Ok(left_value * right_value),
                    NodeType::Div => {
                        if right_value == 0 {
                            Err("Division by zero.".to_string())
                        } else {
                            Ok(left_value / right_value)
                        }
                    }
                    _ => Err("Unsupported operation".to_string()),
                }
            }
            _ => Err("Unsupported node type".to_string()),
        }
    }
    pub fn decode(&mut self, nodes: &Vec<Node>) -> R<(), String> {
        for node in nodes {
            match &node.node_value() {
                NodeType::Assign(var_node, expr_node) => {
                    let value = self.evaluate(expr_node)?;
                    if let NodeType::Variable(var_name) = &var_node.node_value() {
                        self.global_variables.insert(var_name.clone(), value);
                        info!("{} = {}", var_name, value);
                    } else {
                        return Err("Left-hand side of assignment must be a variable.".to_string());
                    }
                }
                _ => {
                    let value = self.evaluate(&Box::new(node.clone()))?;
                    //info!("Result: {}", value);
                }
            }
        }
        Ok(())
    }
}
impl Decoder {
    pub fn new() -> Self {
        Decoder {
            global_variables: HashMap::new(),
            local_variables_stack: Vec::new(),
            func_lists: HashMap::new(),
            last_var_name: None,
        }
    }
}*/
