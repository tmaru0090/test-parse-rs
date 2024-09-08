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
    comment_lists: HashMap<String, (Vec<String>, i32)>,    // コメントとコメント行を保持
}

#[derive(Clone, Debug)]
pub enum VariableValue {
    Int32(i32),
    Str(String),
}
impl Decoder {
    pub fn register_function(
        &mut self,
        name: String,
        args: Vec<String>,
        body: Box<Node>,
    ) -> R<(), String> {
        if self.func_lists.contains_key(&name) {
            return Err(format!("func name {:?} is defined", name));
        }
        self.func_lists.insert(name.clone(), (args, body));
        Ok(())
    }
    pub fn evaluate(&mut self, node: &Box<Node>) -> Result<VariableValue, String> {
        match &node.node_value() {
            NodeType::MultiComment(content,(_,_)) => self.evaluate_comment(content.to_vec()),
            NodeType::Function(func_name, args, body) => {
                self.evaluate_function(func_name, args, body)
            }
            NodeType::Return(ret_value) => self.evaluate_return(ret_value),
            NodeType::Call(func_name, args) => self.evaluate_call(func_name, args),
            NodeType::Number(value) => Ok(VariableValue::Int32(*value)),
            NodeType::String(value) => Ok(VariableValue::Str(value.clone())),
            NodeType::Variable(name) => self.evaluate_variable(name),
            NodeType::Assign(var_node, expr_node) => self.evaluate_assign(var_node, expr_node),
            NodeType::Block(nodes) => self.evaluate_block(nodes),
            NodeType::Add(_left, _right)
            | NodeType::Sub(_left, _right)
            | NodeType::Mul(_left, _right)
            | NodeType::Div(_left, _right) => self.evaluate_binary_op(node),
            _ => Err("Unsupported node type".to_string()),
        }
    }
    fn evaluate_comment(&mut self, content: Vec<String>) -> R<VariableValue, String> {
        self.comment_lists
            .insert(format!("comment{:}", 0), (content, 0));
        Ok(VariableValue::Int32(0))
    }
    fn evaluate_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
    ) -> Result<VariableValue, String> {
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

    fn evaluate_return(&mut self, ret_value: &Box<Node>) -> Result<VariableValue, String> {
        let value = self.evaluate(ret_value)?;
        info!("Return: {:?}", value);
        Ok(value)
    }

    fn evaluate_call(
        &mut self,
        func_name: &String,
        args: &Vec<Node>,
    ) -> Result<VariableValue, String> {
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

    fn evaluate_variable(&mut self, name: &String) -> Result<VariableValue, String> {
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

    fn evaluate_assign(
        &mut self,
        var_node: &Box<Node>,
        expr_node: &Box<Node>,
    ) -> Result<VariableValue, String> {
        let value = self.evaluate(expr_node)?;
        if let NodeType::Variable(var_name) = &var_node.node_value() {
            let mut is_redefined = false;
            for local_vars in &self.local_variables_stack {
                if local_vars.contains_key(var_name) {
                    is_redefined = true;
                    break;
                }
            }
            if self.global_variables.contains_key(var_name) {
                is_redefined = true;
            }
            if is_redefined {
                return Err(format!(
                    "The variable name {:?} is already defined",
                    var_name
                ));
            }
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

    fn evaluate_block(&mut self, nodes: &Vec<Node>) -> Result<VariableValue, String> {
        self.local_variables_stack.push(HashMap::new());
        let mut value = VariableValue::Int32(0);
        for node in nodes {
            value = self.evaluate(&Box::new(node.clone()))?;
        }
        self.local_variables_stack.pop();
        Ok(value)
    }

    fn evaluate_binary_op(&mut self, node: &Box<Node>) -> Result<VariableValue, String> {
        if let NodeType::Add(left, right)
        | NodeType::Sub(left, right)
        | NodeType::Mul(left, right)
        | NodeType::Div(left, right) = &node.node_value()
        {
            let left_value = self.evaluate(left)?;
            let right_value = self.evaluate(right)?;
            match (&node.node_value(), left_value, right_value) {
                (NodeType::Add(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l + r))
                }
                (NodeType::Sub(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l - r))
                }
                (NodeType::Mul(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l * r))
                }
                (NodeType::Div(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    if r == 0 {
                        Err("Division by zero.".to_string())
                    } else {
                        Ok(VariableValue::Int32(l / r))
                    }
                }
                (NodeType::Add(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Str(l + &r))
                }
                _ => Err("Unsupported operation or mismatched types".to_string()),
            }
        } else {
            Err("Unsupported binary operation".to_string())
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
            comment_lists: HashMap::new(),
            last_var_name: None,
        }
    }
}

pub struct AsmInterpreter {
    func_lists: HashMap<String, (Vec<String>, Box<Node>)>,
    global_variables: HashMap<String, VariableValue>,
    local_variables_stack: Vec<HashMap<String, VariableValue>>,
    last_var_name: Option<String>,
}

impl AsmInterpreter {
    pub fn new() -> Self {
        AsmInterpreter {
            func_lists: HashMap::new(),
            global_variables: HashMap::new(),
            local_variables_stack: Vec::new(),
            last_var_name: None,
        }
    }

    pub fn register_function(
        &mut self,
        name: String,
        args: Vec<String>,
        body: Box<Node>,
    ) -> Result<(), String> {
        if self.func_lists.contains_key(&name) {
            return Err(format!("func name {:?} is defined", name));
        }
        self.func_lists.insert(name.clone(), (args, body));
        Ok(())
    }
    pub fn generate_asm(&mut self, nodes: &Vec<Node>) -> Result<String, String> {
        let mut asm_code = String::new();
        for node in nodes {
            asm_code.push_str(&self.generate_asm_node(node)?);
        }
        Ok(asm_code)
    }

    fn generate_asm_node(&mut self, node: &Node) -> Result<String, String> {
        match &node.node_value() {
            NodeType::Function(func_name, args, body) => {
                self.generate_asm_function(func_name, args, body)
            }
            NodeType::Return(ret_value) => self.generate_asm_return(ret_value),
            NodeType::Call(func_name, args) => self.generate_asm_call(func_name, args),
            NodeType::Number(value) => Ok(format!("{}", value)),
            NodeType::String(value) => Ok(format!("\"{}\"", value)),
            NodeType::Variable(name) => Ok(format!("{}", name)),
            NodeType::Assign(var_node, expr_node) => self.generate_asm_assign(var_node, expr_node),
            NodeType::Block(nodes) => self.generate_asm_block(nodes),
            NodeType::Add(left, right)
            | NodeType::Sub(left, right)
            | NodeType::Mul(left, right)
            | NodeType::Div(left, right) => self.generate_asm_binary_op(node),
            _ => Err("Unsupported node type".to_string()),
        }
    }

    fn generate_asm_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
    ) -> Result<String, String> {
        let mut asm_code = format!("{}:\n", func_name);
        for arg in args {
            asm_code.push_str(&format!("  ; argument: {}\n", arg));
        }
        asm_code.push_str(&self.generate_asm(&vec![*body.clone()])?);
        Ok(asm_code)
    }

    fn generate_asm_return(&mut self, ret_value: &Box<Node>) -> Result<String, String> {
        let value = self.generate_asm(&vec![*ret_value.clone()])?;
        Ok(format!("  mov eax, {}\n  ret\n", value))
    }

    fn generate_asm_call(
        &mut self,
        func_name: &String,
        args: &Vec<Node>,
    ) -> Result<String, String> {
        let mut asm_code = String::new();
        for arg in args {
            let arg_value = self.generate_asm(&vec![arg.clone()])?;
            asm_code.push_str(&format!("  push {}\n", arg_value));
        }
        asm_code.push_str(&format!("  call {}\n", func_name));
        Ok(asm_code)
    }

    fn generate_asm_assign(
        &mut self,
        var_node: &Box<Node>,
        expr_node: &Box<Node>,
    ) -> Result<String, String> {
        let value = self.generate_asm(&vec![*expr_node.clone()])?;
        if let NodeType::Variable(var_name) = &var_node.node_value() {
            Ok(format!("  mov {}, {}\n", var_name, value))
        } else {
            Err("Left-hand side of assignment must be a variable.".to_string())
        }
    }

    fn generate_asm_block(&mut self, nodes: &Vec<Node>) -> Result<String, String> {
        let mut asm_code = String::new();
        for node in nodes {
            asm_code.push_str(&self.generate_asm(&vec![node.clone()])?);
        }
        Ok(asm_code)
    }

    fn generate_asm_binary_op(&mut self, node: &Node) -> Result<String, String> {
        if let NodeType::Add(left, right)
        | NodeType::Sub(left, right)
        | NodeType::Mul(left, right)
        | NodeType::Div(left, right) = &node.node_value()
        {
            let left_value = self.generate_asm(&vec![*left.clone()])?;
            let right_value = self.generate_asm(&vec![*right.clone()])?;
            match &node.node_value() {
                NodeType::Add(_, _) => Ok(format!("  add {}, {}\n", left_value, right_value)),
                NodeType::Sub(_, _) => Ok(format!("  sub {}, {}\n", left_value, right_value)),
                NodeType::Mul(_, _) => Ok(format!("  imul {}, {}\n", left_value, right_value)),
                NodeType::Div(_, _) => Ok(format!(
                    "  mov eax, {}\n  idiv {}\n",
                    left_value, right_value
                )),
                _ => Err("Unsupported operation".to_string()),
            }
        } else {
            Err("Unsupported binary operation".to_string())
        }
    }
}
