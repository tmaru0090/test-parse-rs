use crate::parser::Node;
use crate::types::NodeType;
use std::collections::HashMap;
use std::iter::zip;

use anyhow::Result as R;
use log::info;

pub struct Decoder {
    global_variables: HashMap<String, VariableValue>,
    local_variables_stack: Vec<HashMap<String, VariableValue>>,
    func_lists: HashMap<String, (Vec<String>, Box<Node>)>, // 関数の定義を保持
    last_var_name: Option<String>,                         // 最後に代入された変数名を保持
}

#[derive(Clone, Debug)]
pub enum VariableValue {
    Int32(i32),
    Str(String),
}
impl Decoder {
    pub fn evaluate(&mut self, node: &Box<Node>) -> R<VariableValue, String> {
        match &node.node_value() {
            NodeType::Function(func_name, args, body) => {
                if self.func_lists.contains_key(func_name) {
                    return Err(format!(
                        "The name {:?} is defined multiple times",
                        func_name
                    ));
                }
                self.func_lists
                    .insert(func_name.clone(), (args.clone(), body.clone()));

                info!("define function: {:?}", func_name);
                Ok(VariableValue::Int32(0))
            }
            NodeType::Return(ret_value) => {
                let value = self.evaluate(ret_value)?;
                info!("Return: {:?}", value);
                Ok(value)
            }
            NodeType::Call(func_name, args) => {
                if let Some((args_name_v, body)) = self.func_lists.get(func_name).cloned() {
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

                    let result = self.evaluate(&body);

                    self.local_variables_stack.pop();

                    result
                } else {
                    Err(format!("Undefined function: {}", func_name))
                }
            }
            NodeType::Number(value) => Ok(VariableValue::Int32(*value)),
            NodeType::String(value) => Ok(VariableValue::Str(value.clone())),
            NodeType::Variable(name) => {
                for local_vars in &self.local_variables_stack {
                    if local_vars.contains_key(name) {
                        info!(
                            "Local variable: name: {} value: {:?}",
                            name, local_vars[name]
                        );
                        return Ok(local_vars[name].clone());
                    }
                }
                if self.global_variables.contains_key(name) {
                    info!(
                        "Global Variable: name: {} value: {:?}",
                        name, self.global_variables[name]
                    );
                    Ok(self.global_variables[name].clone())
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            NodeType::Assign(var_node, expr_node) => {
                let value = self.evaluate(expr_node)?;
                if let NodeType::Variable(var_name) = &var_node.node_value() {
                    let mut is_redefined = false;
                    // Check in local variables stack
                    for local_vars in &self.local_variables_stack {
                        if local_vars.contains_key(var_name) {
                            is_redefined = true;
                            break;
                        }
                    }
                    // Check in global variables
                    if self.global_variables.contains_key(var_name) {
                        is_redefined = true;
                    }
                    if is_redefined {
                        return Err(format!(
                            "The variable name {:?} is already defined",
                            var_name
                        ));
                    }
                    // Insert into local or global variables
                    if let Some(local_vars) = self.local_variables_stack.last_mut() {
                        local_vars.insert(var_name.clone(), value.clone());
                    } else {
                        self.global_variables
                            .insert(var_name.clone(), value.clone());
                    }
                    self.last_var_name = Some(var_name.clone());
                    info!("Assigning: {} = {:?}", var_name, value);
                } else {
                    return Err("Left-hand side of assignment must be a variable.".to_string());
                }
                Ok(value)
            }
            NodeType::Block(nodes) => {
                self.local_variables_stack.push(HashMap::new());
                let mut value = VariableValue::Int32(0);
                for node in nodes {
                    let current_node: Box<Node> = Box::new(node.clone());
                    value = self.evaluate(&current_node)?;
                }
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
                match (&node.node_value(), left_value, right_value) {
                    (NodeType::Add, VariableValue::Int32(l), VariableValue::Int32(r)) => {
                        Ok(VariableValue::Int32(l + r))
                    }
                    (NodeType::Sub, VariableValue::Int32(l), VariableValue::Int32(r)) => {
                        Ok(VariableValue::Int32(l - r))
                    }
                    (NodeType::Mul, VariableValue::Int32(l), VariableValue::Int32(r)) => {
                        Ok(VariableValue::Int32(l * r))
                    }
                    (NodeType::Div, VariableValue::Int32(l), VariableValue::Int32(r)) => {
                        if r == 0 {
                            Err("Division by zero.".to_string())
                        } else {
                            Ok(VariableValue::Int32(l / r))
                        }
                    }

                    (NodeType::Add, VariableValue::Str(l), VariableValue::Str(r)) => {
                        Ok(VariableValue::Str(l+&r))
                    }
                    _ => Err("Unsupported operation or mismatched types".to_string()),
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
                        self.global_variables
                            .insert(var_name.clone(), value.clone());
                        info!("{} = {:?}", var_name, value);
                    } else {
                        return Err("Left-hand side of assignment must be a variable.".to_string());
                    }
                }
                _ => {
                    let value = self.evaluate(&Box::new(node.clone()))?;
                    //info!("Result: {:?}", value);
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
