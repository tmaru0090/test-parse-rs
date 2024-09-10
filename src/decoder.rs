use crate::custom_compile_error;
use crate::parser::Node;
use crate::types::NodeValue;
use anyhow::{Context, Result as R};
use log::{error, info};
use property_rs::Property;
use std::collections::HashMap;
use std::iter::zip;
#[derive(Debug, Clone, Property)]
pub struct Decoder {
    #[property(get)]
    global_variables: HashMap<String, VariableValue>, // グローバルスコープ

    #[property(get)]
    func_lists: HashMap<
        String, // 関数名
        (
            Vec<String>,                              // 引数名のリスト
            Box<Node>,                                // 関数実装(AST)
            VariableValue,                            // 関数戻り値
            Vec<Vec<HashMap<String, VariableValue>>>, // 関数ローカルスタック
        ),
    >,
    #[property(get)]
    last_var_name: Option<String>,

    #[property(get)]
    comment_lists: HashMap<String, (Vec<String>, i32)>,

    #[property(get)]
    current_node: Option<Box<Node>>, // 現在のノードを保持
    #[property(get)]
    current_function: Option<String>, // 現在の関数名を保持するフィールドを追加
    #[property(get)]
    input: String,
}

#[derive(Clone, Debug)]
pub enum VariableValue {
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Str(String),
    Bool(bool),
    Unit(()),
}

impl Decoder {
    pub fn evaluate(&mut self, node: &Box<Node>) -> R<VariableValue, String> {
        self.current_node = Some(node.clone());
        //info!("current_node: {:?}", self.current_node.clone());

        match &node.node_value() {
            NodeValue::If(condition, body) => self.evaluate_if_statement(condition, body),
            NodeValue::While(condition, body) => self.evaluate_while_statement(condition, body),
            NodeValue::MultiComment(content, (_, _)) => {
                self.evaluate_multi_comment(content.to_vec())
            }
            NodeValue::SingleComment(content, (_, _)) => self.evaluate_single_comment(content),
            NodeValue::Function(func_name, args, body, ret_value) => {
                self.evaluate_function(func_name, args, body, ret_value)
            }
            NodeValue::Return(ret_value) => {
                if self.is_in_function_scope() {
                    let ret = self.evaluate_return(ret_value);
                    info!("Return: {:?}", ret);
                    self.pop_local_stack_frame();
                    ret
                } else {
                    let node = match self.current_node.clone() {
                        Some(v) => v,
                        _ => todo!(),
                    };
                    return Err(custom_compile_error!(
                        node.line(),
                        node.column(),
                        &self.input(),
                        "Return statement outside of function scope"
                    ));
                }
            }
            NodeValue::Call(func_name, args) => self.evaluate_call(node, func_name, args),
            NodeValue::Number(value) => Ok(VariableValue::Int32(*value)),
            NodeValue::Bool(value) => Ok(VariableValue::Bool(*value)),
            NodeValue::String(value) => Ok(VariableValue::Str(value.clone())),
            NodeValue::Assign(var_node, expr_node) => self.evaluate_assign(var_node, expr_node),
            NodeValue::Block(nodes) => self.evaluate_block(nodes),
            NodeValue::Variable(name) => self.evaluate_variable(name),
            NodeValue::VariableDeclaration(var_node, expr_node) => {
                self.evaluate_variable_declaration(var_node, expr_node)
            }
            NodeValue::Add(_left, _right)
            | NodeValue::Sub(_left, _right)
            | NodeValue::Mul(_left, _right)
            | NodeValue::Div(_left, _right) => self.evaluate_binary_op(node),
            NodeValue::Empty | NodeValue::StatementEnd => Ok(VariableValue::Unit(())), // 空のノードをデフォルト値で処理

            _ => {
                let node = match self.current_node.clone() {
                    Some(v) => v,
                    _ => todo!(),
                };
                return Err(custom_compile_error!(
                    node.line(),
                    node.column(),
                    &self.input(),
                    "Unsupported node type: {:?}",
                    node.node_value()
                ));
            }
        }
    }

    pub fn evaluate_assign(
        &mut self,
        var_node: &Box<Node>,
        expr_node: &Box<Node>,
    ) -> R<VariableValue, String> {
        let value = self.evaluate(expr_node)?;
        if let NodeValue::Variable(var_name) = &var_node.node_value() {
            let mut assigned = false;

            // 関数ごとのローカルスタックをチェック
            for local_vars_stack in self
                .func_lists
                .values_mut()
                .flat_map(|(_, _, _, stack)| stack.iter_mut())
            {
                for local_vars in local_vars_stack {
                    if local_vars.contains_key(var_name) {
                        local_vars.insert(var_name.clone(), value.clone());
                        info!(
                            "Assigned Local Variable: name: {} value: {:?}",
                            var_name, value
                        );
                        assigned = true;
                        break;
                    }
                }
                if assigned {
                    break;
                }
            }

            // グローバル変数をチェック
            if !assigned && self.global_variables.contains_key(var_name) {
                self.global_variables
                    .insert(var_name.clone(), value.clone());
                info!(
                    "Assigned Global Variable: name: {} value: {:?}",
                    var_name, value
                );
                assigned = true;
            }

            if !assigned {
                let node = match self.current_node.clone() {
                    Some(v) => v,
                    _ => todo!(),
                };

                return Err(custom_compile_error!(
                    node.line(),
                    node.column(),
                    &self.input(),
                    "Variable {:?} not found in any scope",
                    var_name
                ));
            }

            self.last_var_name = Some(var_name.clone());
        } else {
            let node = match self.current_node.clone() {
                Some(v) => v,
                _ => todo!(),
            };

            return Err(custom_compile_error!(
                node.line(),
                node.column(),
                &self.input(),
                "Left-hand side of assignment must be a variable."
            ));
        }
        Ok(value)
    }
    fn evaluate_variable_declaration(
        &mut self,
        var_node: &Box<Node>,
        expr_node: &Box<Node>,
    ) -> R<VariableValue, String> {
        let var_name = match &var_node.node_value() {
            NodeValue::Variable(name) => name.clone(),
            _ => {
                return Err(custom_compile_error!(
                    var_node.line(),
                    var_node.column(),
                    &self.input(),
                    "Expected a variable name in variable declaration"
                ));
            }
        };

        let value = self.evaluate(expr_node)?;
        let mut is_redefined = false;

        // 関数ごとのローカルスタックをチェック
        for local_vars_stack in self
            .func_lists
            .values()
            .flat_map(|(_, _, _, stack)| stack.iter())
        {
            for local_vars in local_vars_stack {
                if local_vars.contains_key(&var_name) {
                    is_redefined = true;
                    break;
                }
            }
            if is_redefined {
                break;
            }
        }
        if self.global_variables.contains_key(&var_name) {
            is_redefined = true;
        }

        if is_redefined {
            let node = match self.current_node.clone() {
                Some(v) => v,
                _ => todo!(),
            };

            return Err(custom_compile_error!(
                node.line(),
                node.column(),
                &self.input(),
                "The variable name {:?} is already defined",
                var_name
            ));
        }

        // 現在の関数のローカルスタックに変数を追加
        if let Some(local_vars_stack) = self
            .func_lists
            .values_mut()
            .flat_map(|(_, _, _, stack)| stack.last_mut())
            .next()
        {
            if let Some(local_vars) = local_vars_stack.last_mut() {
                local_vars.insert(var_name.clone(), value.clone());
                info!(
                    "Declare Local Variable: name: {} value: {:?} in function: {:?}",
                    var_name.clone(),
                    value.clone(),
                    self.current_function.clone()
                );
            }
        } else {
            self.global_variables
                .insert(var_name.clone(), value.clone());
            info!(
                "Declare Global Variable: name: {} value: {:?} in function: {:?}",
                var_name.clone(),
                value.clone(),
                self.current_function().clone()
            );
        }

        self.last_var_name = Some(var_name.clone());
        Ok(value)
    }

    fn evaluate_block(&mut self, nodes: &Vec<Box<Node>>) -> R<VariableValue, String> {
        let current_function = self.current_function.clone();
        let mut func_lists = self.func_lists();
        let stack = if let Some(func_name) = current_function.clone() {
            func_lists
                .get_mut(&func_name)
                .and_then(|(_, _, _, stacks)| stacks.last_mut())
        } else {
            None
        };

        if let Some(local_stack) = stack {
            local_stack.push(HashMap::new());
            info!(
                "in {}: Local stack before evaluation: {:?}",
                current_function.clone().unwrap_or("unknown".to_string()),
                local_stack
            );
            let mut value = VariableValue::Unit(());

            let nodes_clone = nodes.clone();

            for node in nodes_clone {
                let node_clone = node.clone();
                value = self.evaluate(&node_clone)?;
                if let NodeValue::Return(_) = node.node_value() {
                    local_stack.pop();
                    info!(
                        "in {}: Local stack after pop (return): {:?}",
                        current_function.clone().unwrap_or("unknown".to_string()),
                        local_stack
                    );
                    return Ok(value);
                }
            }
            info!(
                "in {}: Local stack before evaluation: {:?}",
                current_function.clone().unwrap_or("unknown".to_string()),
                local_stack
            );
            local_stack.pop();
            Ok(value)
        } else {
            let node = match self.current_node.clone() {
                Some(v) => v,
                _ => todo!(),
            };
            return Err(custom_compile_error!(
                node.line(),
                node.column(),
                &self.input(),
                "No local stack found for the current function"
            ));
        }
    }
    fn pop_local_stack_frame(&mut self) {
        if let Some(local_vars_stack) = self
            .func_lists
            .values_mut()
            .flat_map(|(_, _, _, stack)| stack.last_mut())
            .next()
        {
            local_vars_stack.pop();
        }
    }

    fn evaluate_call(
        &mut self,
        node: &Box<Node>,
        func_name: &String,
        args: &Vec<Node>,
    ) -> R<VariableValue, String> {
        // 一時的に必要なデータをコピー
        let (param_names, body, local_stack) = {
            if let Some((param_names, body, _, local_stack)) = self.func_lists.get(func_name) {
                (param_names.clone(), body.clone(), local_stack.clone())
            } else {
                return Err(custom_compile_error!(
                    node.line(),
                    node.column(),
                    &self.input(),
                    "Function {:?} not defined",
                    func_name
                ));
            }
        };

        if args.len() != param_names.len() {
            return Err(custom_compile_error!(
                node.line(),
                node.column(),
                &self.input(),
                "Function {:?} expects {:?} arguments, but {:?} were provided",
                func_name,
                param_names.len(),
                args.len()
            ));
        }

        let mut local_vars = HashMap::new();
        for (param_name, arg) in param_names.iter().zip(args.iter()) {
            let value = self.evaluate(&Box::new(arg.clone()))?;
            local_vars.insert(param_name.clone(), value);
        }

        // ミュータブルな借用を再度取得してローカルスタックを更新
        if let Some((_, _, _, local_stack)) = self.func_lists.get_mut(func_name) {
            local_stack.push(vec![local_vars]);
        }

        self.current_function = Some(func_name.clone());
        let result = self.evaluate(&body);

        // ミュータブルな借用を再度取得してローカルスタックをポップ
        if let Some((_, _, _, local_stack)) = self.func_lists.get_mut(func_name) {
            local_stack.pop();
        }

        self.current_function = None;

        result
    }

    fn evaluate_return(&mut self, ret_value: &Box<Node>) -> R<VariableValue, String> {
        let value = self.evaluate(ret_value)?;
        if self.is_in_function_scope() {
            self.current_node = None;
            return Ok(value);
        }
        Ok(value)
    }

    pub fn evaluate_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
        ret_value: &Box<Node>,
    ) -> R<VariableValue, String> {
        // まず関数のシグネチャだけを登録
        self.register_function_signature(func_name.clone(), args.clone())?;
        info!("define function signature: {:?}", func_name);

        // 次に関数のボディを評価
        let ret_value = self.evaluate(ret_value)?;
        self.register_function_body(func_name.clone(), body.clone(), ret_value.clone())?;
        info!("define function body: {:?}", func_name);

        Ok(VariableValue::Unit(()))
    }

    fn register_function_signature(&mut self, name: String, args: Vec<String>) -> R<(), String> {
        if self.func_lists.contains_key(&name) {
            let node = match self.current_node.clone() {
                Some(v) => v,
                _ => todo!(),
            };
            return Err(custom_compile_error!(
                node.line(),
                node.column(),
                &self.input(),
                "Function name {:?} is already defined",
                name
            ));
        }
        self.func_lists.insert(
            name.clone(),
            (
                args,
                Box::new(Node::new(NodeValue::Empty, None, 0, 0)),
                VariableValue::Unit(()),
                Vec::new(),
            ),
        );
        Ok(())
    }

    fn register_function_body(
        &mut self,
        name: String,
        body: Box<Node>,
        ret_value: VariableValue,
    ) -> R<(), String> {
        // 一時的に必要なデータをコピー
        let (args, mut local_stack) = {
            if let Some((args, _, _, local_stack)) = self.func_lists.get(&name) {
                (args.clone(), local_stack.clone())
            } else {
                let node = match self.current_node.clone() {
                    Some(v) => v,
                    _ => todo!(),
                };
                return Err(custom_compile_error!(
                    node.line(),
                    node.column(),
                    &self.input(),
                    "Function name {:?} is not defined",
                    name
                ));
            }
        };

        // ミュータブルな借用を再度取得して関数のボディを登録
        if let Some((_, _, _, local_stack_ref)) = self.func_lists.get_mut(&name) {
            *local_stack_ref = Vec::new();
            local_stack = local_stack_ref.clone();
        }

        self.func_lists
            .insert(name.clone(), (args, body, ret_value, local_stack));

        Ok(())
    }
    fn is_in_function_scope(&self) -> bool {
        self.func_lists
            .values()
            .any(|(_, _, _, stack)| !stack.is_empty())
    }
    pub fn evaluate_if_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> R<VariableValue, String> {
        let bool_value = self.evaluate_binary_op(condition)?;
        if let VariableValue::Bool(value) = bool_value {
            if value {
                return Ok(self.evaluate(&body)?);
            }
        }
        Ok(VariableValue::Bool(false))
    }
    pub fn evaluate_while_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> R<VariableValue, String> {
        let bool_value = self.evaluate_binary_op(condition)?;
        if let VariableValue::Bool(value) = bool_value {
            while value {
                return Ok(self.evaluate(&body)?);
            }
        }
        Ok(VariableValue::Bool(false))
    }

    fn evaluate_multi_comment(&mut self, content: Vec<String>) -> R<VariableValue, String> {
        self.comment_lists
            .insert(format!("comment: {:}", 0), (content, 0));
        Ok(VariableValue::Unit(()))
    }
    fn evaluate_single_comment(&mut self, content: &String) -> R<VariableValue, String> {
        self.comment_lists.insert(
            format!("comment: {:}", 0),
            (vec![(*content.clone()).to_string()], 0),
        );

        Ok(VariableValue::Unit(()))
    }

    fn evaluate_variable(&mut self, name: &String) -> R<VariableValue, String> {
        for local_vars_stack in self
            .func_lists
            .values()
            .flat_map(|(_, _, _, stack)| stack.iter())
        {
            for local_vars in local_vars_stack {
                if local_vars.contains_key(name) {
                    info!("Found variable {} in local stack: {:?}", name, local_vars);
                    return Ok(local_vars[name].clone());
                }
            }
        }
        if self.global_variables.contains_key(name) {
            info!(
                "Found variable {} in global scope: {:?}",
                name,
                self.global_variables[name].clone()
            );
            Ok(self.global_variables[name].clone())
        } else {
            return Ok(VariableValue::Int32(0));
        }
    }
    pub fn decode(&mut self, nodes: &Vec<Box<Node>>) -> R<(), String> {
        for node in nodes {
            match &node.node_value() {
                NodeValue::Assign(var_node, expr_node) => {
                    let value = self.evaluate(expr_node)?;
                    if let NodeValue::Variable(var_name) = &var_node.node_value() {
                        self.global_variables
                            .insert(var_name.clone(), value.clone());
                        info!(
                            "Assign Global Variable: name: {} value: {:?}",
                            var_name, value
                        );
                    } else {
                        let node = match self.current_node.clone() {
                            Some(v) => v,
                            _ => todo!(),
                        };

                        return Err(custom_compile_error!(
                            node.line(),
                            node.column(),
                            &self.input(),
                            "Left-hand side of assignment must be a variable."
                        ));
                    }
                }
                _ => {
                    let value = self.evaluate(&Box::new(*node.clone()))?;
                    // 他の処理が必要ならここに追加
                }
            }
        }
        Ok(())
    }
    fn evaluate_binary_op(&mut self, node: &Box<Node>) -> R<VariableValue, String> {
        // 条件演算子の処理
        if let NodeValue::Eq(left, right)
        | NodeValue::Ne(left, right)
        | NodeValue::Lt(left, right)
        | NodeValue::Gt(left, right)
        | NodeValue::Le(left, right)
        | NodeValue::Ge(left, right) = &node.node_value()
        {
            let left_value = self.evaluate(left)?;
            let right_value = self.evaluate(right)?;

            match (&node.node_value(), left_value, right_value) {
                // 等しい (==)
                (NodeValue::Eq(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l == r))
                }
                (NodeValue::Eq(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Bool(l == r))
                }
                // 等しくない (!=)
                (NodeValue::Ne(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l != r))
                }
                (NodeValue::Ne(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Bool(l != r))
                }
                // 小なり (<)
                (NodeValue::Lt(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l < r))
                }
                // 大なり (>)
                (NodeValue::Gt(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l > r))
                }
                // 以下 (<=)
                (NodeValue::Le(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l <= r))
                }
                // 以上 (>=)
                (NodeValue::Ge(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l >= r))
                }
                _ => Err("Unsupported operation or mismatched types in condition".to_string()),
            }
        }
        // 通常の演算子の処理
        else if let NodeValue::Add(left, right)
        | NodeValue::Sub(left, right)
        | NodeValue::Mul(left, right)
        | NodeValue::Div(left, right) = &node.node_value()
        {
            let left_value = self.evaluate(left)?;
            let right_value = self.evaluate(right)?;
            match (&node.node_value(), left_value, right_value) {
                (NodeValue::Add(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l + r))
                }
                (NodeValue::Sub(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l - r))
                }
                (NodeValue::Mul(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l * r))
                }
                (NodeValue::Div(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    if r == 0 {
                        Err("Division by zero.".to_string())
                    } else {
                        Ok(VariableValue::Int32(l / r))
                    }
                }
                (NodeValue::Add(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Str(l + &r))
                }
                _ => Err("Unsupported operation or mismatched types".to_string()),
            }
        } else {
            Err("Unsupported binary operation".to_string())
        }
    }
}
impl Decoder {
    pub fn new(input: String) -> Self {
        Decoder {
            global_variables: HashMap::new(),
            func_lists: HashMap::new(),
            comment_lists: HashMap::new(),
            last_var_name: None,
            current_node: None,
            current_function: None,
            input: input.clone(),
        }
    }
}

pub struct AsmInterpreter {
    global_variables: HashMap<String, VariableValue>,
    func_lists: HashMap<
        String,
        (
            Vec<String>,
            Box<Node>,
            VariableValue,
            Vec<Vec<HashMap<String, VariableValue>>>,
        ),
    >,
    last_var_name: Option<String>,
    comment_lists: HashMap<String, (Vec<String>, i32)>,
    current_node: Option<Box<Node>>,
    current_function: Option<String>,
    input: String,
}
impl AsmInterpreter {
    pub fn new(input: String) -> Self {
        AsmInterpreter {
            global_variables: HashMap::new(),
            func_lists: HashMap::new(),
            comment_lists: HashMap::new(),
            last_var_name: None,
            current_node: None,
            current_function: None,
            input,
        }
    }

    pub fn generate_asm(&mut self, nodes: &Vec<Node>) -> String {
        let mut asm_code = String::new();
        for node in nodes {
            asm_code.push_str(&self.evaluate(&Box::new(node.clone())));
        }
        asm_code
    }

    fn evaluate(&mut self, node: &Box<Node>) -> String {
        match &node.node_value() {
            NodeValue::Assign(var_node, expr_node) => self.evaluate_assign(var_node, expr_node),
            NodeValue::If(condition, body) => self.evaluate_if_statement(condition, body),
            NodeValue::Function(func_name, args, body, ret_value) => {
                self.evaluate_function(func_name, args, body, ret_value)
            }
            NodeValue::Return(ret_value) => self.evaluate_return(ret_value),
            NodeValue::Call(func_name, args) => self.evaluate_call(func_name, args),
            NodeValue::Number(value) => format!("mov eax, {}\n", value),
            NodeValue::String(value) => format!("mov eax, '{}'\n", value),
            NodeValue::Add(left, right) => self.evaluate_binary_op("add", left, right),
            NodeValue::Sub(left, right) => self.evaluate_binary_op("sub", left, right),
            NodeValue::Mul(left, right) => self.evaluate_binary_op("mul", left, right),
            NodeValue::Div(left, right) => self.evaluate_binary_op("div", left, right),
            _ => String::new(),
        }
    }

    fn evaluate_assign(&mut self, var_node: &Box<Node>, expr_node: &Box<Node>) -> String {
        let value = self.evaluate(expr_node);
        if let NodeValue::Variable(var_name) = &var_node.node_value() {
            format!("mov {}, eax\n", var_name)
        } else {
            String::new()
        }
    }

    fn evaluate_if_statement(&mut self, condition: &Box<Node>, body: &Box<Node>) -> String {
        let condition_code = self.evaluate(condition);
        let body_code = self.evaluate(body);
        format!("cmp eax, 1\nje _if_body\n_if_body:\n{}", body_code)
    }

    fn evaluate_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
        ret_value: &Box<Node>,
    ) -> String {
        let body_code = self.evaluate(body);
        let ret_code = self.evaluate(ret_value);
        format!("{}:\n{}\nret\n", func_name, body_code + &ret_code)
    }

    fn evaluate_return(&mut self, ret_value: &Box<Node>) -> String {
        self.evaluate(ret_value)
    }

    fn evaluate_call(&mut self, func_name: &String, args: &Vec<Node>) -> String {
        let mut args_code = String::new();
        for arg in args {
            args_code.push_str(&self.evaluate(&Box::new(arg.clone())));
        }
        format!("call {}\n", func_name)
    }

    fn evaluate_binary_op(&mut self, op: &str, left: &Box<Node>, right: &Box<Node>) -> String {
        let left_code = self.evaluate(left);
        let right_code = self.evaluate(right);
        format!("{}\n{}\n{} eax, ebx\n", left_code, right_code, op)
    }
}

/*
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
    ) -> R<(), String> {
        if self.func_lists.contains_key(&name) {
            return Err(format!("func name {:?} is defined", name));
        }
        self.func_lists.insert(name.clone(), (args, body));
        Ok(())
    }
    pub fn generate_asm(&mut self, nodes: &Vec<Node>) -> R<String, String> {
        let mut asm_code = String::new();
        for node in nodes {
            asm_code.push_str(&self.generate_asm_node(node)?);
        }
        Ok(asm_code)
    }

    fn generate_asm_node(&mut self, node: &Node) -> R<String, String> {
        match &node.node_value() {
            NodeValue::Function(func_name, args, body, ret_value) => {
                self.generate_asm_function(func_name, args, body)
            }
            NodeValue::Return(ret_value) => self.generate_asm_return(ret_value),
            NodeValue::Call(func_name, args) => self.generate_asm_call(func_name, args),
            NodeValue::Number(value) => Ok(format!("{}", value)),
            NodeValue::String(value) => Ok(format!("\"{}\"", value)),
            NodeValue::Variable(name) => Ok(format!("{}", name)),
            NodeValue::Assign(var_node, expr_node) => self.generate_asm_assign(var_node, expr_node),
            NodeValue::Block(nodes) => self.generate_asm_block(nodes),
            NodeValue::Add(left, right)
            | NodeValue::Sub(left, right)
            | NodeValue::Mul(left, right)
            | NodeValue::Div(left, right) => self.generate_asm_binary_op(node),
            _ => Err("Unsupported node type".to_string()),
        }
    }

    fn generate_asm_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
    ) -> R<String, String> {
        let mut asm_code = format!("{}:\n", func_name);
        for arg in args {
            asm_code.push_str(&format!("  ; argument: {}\n", arg));
        }
        asm_code.push_str(&self.generate_asm(&vec![*body.clone()])?);
        Ok(asm_code)
    }

    fn generate_asm_return(&mut self, ret_value: &Box<Node>) -> R<String, String> {
        let value = self.generate_asm(&vec![*ret_value.clone()])?;
        Ok(format!("  mov eax, {}\n  ret\n", value))
    }

    fn generate_asm_call(
        &mut self,
        func_name: &String,
        args: &Vec<Node>,
    ) -> R<String, String> {
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
    ) -> R<String, String> {
        let value = self.generate_asm(&vec![*expr_node.clone()])?;
        if let NodeValue::Variable(var_name) = &var_node.node_value() {
            Ok(format!("  mov {}, {}\n", var_name, value))
        } else {
            Err("Left-hand side of assignment must be a variable.".to_string())
        }
    }

    fn generate_asm_block(&mut self, nodes: &Vec<Node>) -> R<String, String> {
        let mut asm_code = String::new();
        for node in nodes {
            asm_code.push_str(&self.generate_asm(&vec![node.clone()])?);
        }
        Ok(asm_code)
    }

    fn generate_asm_binary_op(&mut self, node: &Node) -> R<String, String> {
        if let NodeValue::Add(left, right)
        | NodeValue::Sub(left, right)
        | NodeValue::Mul(left, right)
        | NodeValue::Div(left, right) = &node.node_value()
        {
            let left_value = self.generate_asm(&vec![*left.clone()])?;
            let right_value = self.generate_asm(&vec![*right.clone()])?;
            match &node.node_value() {
                NodeValue::Add(_, _) => Ok(format!("  add {}, {}\n", left_value, right_value)),
                NodeValue::Sub(_, _) => Ok(format!("  sub {}, {}\n", left_value, right_value)),
                NodeValue::Mul(_, _) => Ok(format!("  imul {}, {}\n", left_value, right_value)),
                NodeValue::Div(_, _) => Ok(format!(
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
*/

/*
#[derive(Debug, Clone, Property)]
pub struct Decoder {
    #[property(get)]
    global_variables: HashMap<String, VariableValue>, // グローバルスコープ

    #[property(get)]
    func_lists: HashMap<
        String, // 関数名
        (
            Vec<String>,                              // 引数名のリスト
            Box<Node>,                                // 関数実装(AST)
            Box<Node>,                                // 関数戻り値(AST)
            Vec<Vec<HashMap<String, VariableValue>>>, // 関数ローカルスタック
        ),
    >,
    #[property(get)]
    last_var_name: Option<String>,

    #[property(get)]
    comment_lists: HashMap<String, (Vec<String>, i32)>,

    #[property(get)]
    current_node: Option<Box<Node>>, // 現在のノードを保持
    #[property(get)]
    current_function: Option<String>, // 現在の関数名を保持するフィールドを追加
}

#[derive(Clone, Debug)]
pub enum VariableValue {
    Int32(i32),
    Str(String),
    Bool(bool),
}
impl Decoder {
    pub fn register_function(
        &mut self,
        name: String,
        args: Vec<String>,
        body: Box<Node>,
        ret_value: Box<Node>,
    ) -> R<(), String> {
        if self.func_lists.contains_key(&name) {
            return Err(format!("func name {:?} is defined", name));
        }
        self.func_lists
            .insert(name.clone(), (args, body, ret_value, Vec::new()));
        Ok(())
    }
    pub fn evaluate(&mut self, node: &Box<Node>) -> R<VariableValue, String> {
        match &node.node_value() {
            NodeValue::If(condition, body) => self.evaluate_if_statement(condition, body),
            NodeValue::MultiComment(content, (_, _)) => self.evaluate_comment(content.to_vec()),
            NodeValue::Function(func_name, args, body, ret_value) => {
                self.evaluate_function(func_name, args, body, ret_value)
            }
            NodeValue::Return(ret_value) => {
                if self.is_in_function_scope() {
                    info!("Return: {:?}", ret_value);
                    return self.evaluate_return(ret_value);
                } else {
                    Err("Return statement outside of function scope".to_string())
                }
            }
            NodeValue::Call(func_name, args) => self.evaluate_call(node, func_name, args),
            NodeValue::Number(value) => Ok(VariableValue::Int32(*value)),
            NodeValue::String(value) => Ok(VariableValue::Str(value.clone())),
            NodeValue::Variable(name) => self.evaluate_variable(name),
            NodeValue::Assign(var_node, expr_node) => self.evaluate_assign(var_node, expr_node),
            NodeValue::Block(nodes) => self.evaluate_block(nodes),
            NodeValue::Add(_left, _right)
            | NodeValue::Sub(_left, _right)
            | NodeValue::Mul(_left, _right)
            | NodeValue::Div(_left, _right) => self.evaluate_binary_op(node),
            NodeValue::Empty => Ok(VariableValue::Int32(0)), // 空のノードをデフォルト値で処理
            _ => {
                // デバッグ用のログを追加
                error!("Unsupported node type: {:?}", node.node_value());
                Err("Unsupported node type".to_string())
            }
        }
    }
    fn evaluate_block(&mut self, nodes: &Vec<Node>) -> R<VariableValue, String> {
        let current_function = self.current_function.clone();
        let mut func_lists = self.func_lists();

        // 現在の関数名を使用してローカルスタックを取得
        let stack = if let Some(func_name) = current_function {
            func_lists
                .get_mut(&func_name)
                .and_then(|(_, _, _, stacks)| stacks.last_mut())
        } else {
            None
        };

        if let Some(local_stack) = stack {
            local_stack.push(HashMap::new());
            let mut value = VariableValue::Int32(0);

            let nodes_clone = nodes.clone();

            for node in nodes_clone {
                let node_clone = node.clone();
                value = self.evaluate(&Box::new(node_clone))?;

                if let NodeValue::Return(_) = node.node_value() {
                    //local_stack.pop();
                    return Ok(value);
                }
            }
            local_stack.pop();
            Ok(value)
        } else {
            Err("No local stack found for the current function".to_string())
        }
    }
    fn evaluate_call(
        &mut self,
        node: &Box<Node>,
        func_name: &String,
        args: &Vec<Node>,
    ) -> R<VariableValue, String> {
        // 必要なデータを一時変数に格納
        let (args_name_v, body, ret_value, local_stack) =
            if let Some(data) = self.func_lists.get(func_name) {
                (
                    data.0.clone(),
                    data.1.clone(),
                    data.2.clone(),
                    data.3.clone(),
                )
            } else {
                return Err(format!("Undefined function: {}", func_name));
            };

        let mut local_vars = HashMap::new();
        for (arg_name, arg) in args_name_v.iter().zip(args.iter()) {
            let arg_clone = arg.clone();
            let arg_value = self.evaluate(&Box::new(arg_clone))?;
            local_vars.insert(arg_name.clone(), arg_value);
        }

        // ローカル変数をスタックに追加
        if let Some(func_data) = self.func_lists.get_mut(func_name) {
            func_data.3.push(vec![local_vars]);
        }

        // 現在の関数名を設定
        self.current_function = Some(func_name.clone());

        // 関数の実行状態を保存
        let previous_node = self.current_node.clone();
        self.current_node = Some(node.clone());

        // 関数本体を評価
        let result = self.evaluate(&body)?;

        // 関数の実行状態を復元
        self.current_node = previous_node;

        let return_value = self.evaluate(&ret_value)?;
        // ローカルスタックから変数を削除
        if let Some(func_data) = self.func_lists.get_mut(func_name) {
            func_data.3.pop();
        }

        // 現在の関数名をクリア
        self.current_function = None;
        Ok(return_value)
    }

    fn evaluate_assign(
        &mut self,
        var_node: &Box<Node>,
        expr_node: &Box<Node>,
    ) -> R<VariableValue, String> {
        let value = self.evaluate(expr_node)?;
        if let NodeValue::Variable(var_name) = &var_node.node_value() {
            let mut is_redefined = false;

            // 関数ごとのローカルスタックをチェック
            for local_vars_stack in self
                .func_lists
                .values()
                .flat_map(|(_, _, _, stack)| stack.iter())
            {
                for local_vars in local_vars_stack {
                    if local_vars.contains_key(var_name) {
                        is_redefined = true;
                        break;
                    }
                }
                if is_redefined {
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

            // 現在の関数のローカルスタックに変数を追加
            if let Some(local_vars_stack) = self
                .func_lists
                .values_mut()
                .flat_map(|(_, _, _, stack)| stack.last_mut())
                .next()
            {
                if let Some(local_vars) = local_vars_stack.last_mut() {
                    local_vars.insert(var_name.clone(), value.clone());
                }
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

    fn evaluate_return(&mut self, ret_value: &Box<Node>) -> R<VariableValue, String> {
        let value = self.evaluate(ret_value)?;
        // 関数の呼び出し元に戻る
        if self.is_in_function_scope() {
            self.current_node = None;
            return Ok(value);
        }
        Ok(value)
    }
    pub fn evaluate_function(
        &mut self,
        func_name: &String,
        args: &Vec<String>,
        body: &Box<Node>,
        ret_value: &Box<Node>,
    ) -> R<VariableValue, String> {
        self.register_function(
            func_name.clone(),
            args.clone(),
            body.clone(),
            ret_value.clone(),
        )?;
        info!("define function: {:?}", func_name);
        Ok(VariableValue::Int32(0))
    }
    fn is_in_function_scope(&self) -> bool {
        self.func_lists
            .values()
            .any(|(_, _, _, stack)| !stack.is_empty())
    }
    pub fn evaluate_if_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> R<VariableValue, String> {
        let bool_value = self.evaluate_binary_op(condition)?;
        if let VariableValue::Bool(value) = bool_value {
            if value {
                return Ok(self.evaluate(&body)?);
            }
        }
        Ok(VariableValue::Bool(false))
    }

    fn evaluate_comment(&mut self, content: Vec<String>) -> R<VariableValue, String> {
        self.comment_lists
            .insert(format!("comment{:}", 0), (content, 0));
        Ok(VariableValue::Int32(0))
    }

    fn evaluate_variable(&mut self, name: &String) -> R<VariableValue, String> {
        for local_vars_stack in self
            .func_lists
            .values()
            .flat_map(|(_, _, _, stack)| stack.iter())
        {
            for local_vars in local_vars_stack {
                if local_vars.contains_key(name) {
                    info!(
                        "Local variable: name: {} value: {:?}",
                        name, local_vars[name]
                    );
                    return Ok(local_vars[name].clone());
                }
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
    pub fn decode(&mut self, nodes: &Vec<Node>) -> R<(), String> {
        for node in nodes {
            match &node.node_value() {
                NodeValue::Assign(var_node, expr_node) => {
                    let value = self.evaluate(expr_node)?;
                    if let NodeValue::Variable(var_name) = &var_node.node_value() {
                        self.global_variables
                            .insert(var_name.clone(), value.clone());
                        info!("{} = {:?}", var_name, value);
                    } else {
                        return Err("Left-hand side of assignment must be a variable.".to_string());
                    }
                }
                _ => {
                    let value = self.evaluate(&Box::new(node.clone()))?;
                    // 他の処理が必要ならここに追加
                }
            }
        }
        Ok(())
    }
    fn evaluate_binary_op(&mut self, node: &Box<Node>) -> R<VariableValue, String> {
        // 条件演算子の処理
        if let NodeValue::Eq(left, right)
        | NodeValue::Ne(left, right)
        | NodeValue::Lt(left, right)
        | NodeValue::Gt(left, right)
        | NodeValue::Le(left, right)
        | NodeValue::Ge(left, right) = &node.node_value()
        {
            let left_value = self.evaluate(left)?;
            let right_value = self.evaluate(right)?;

            match (&node.node_value(), left_value, right_value) {
                // 等しい (==)
                (NodeValue::Eq(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l == r))
                }
                (NodeValue::Eq(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Bool(l == r))
                }
                // 等しくない (!=)
                (NodeValue::Ne(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l != r))
                }
                (NodeValue::Ne(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Bool(l != r))
                }
                // 小なり (<)
                (NodeValue::Lt(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l < r))
                }
                // 大なり (>)
                (NodeValue::Gt(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l > r))
                }
                // 以下 (<=)
                (NodeValue::Le(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l <= r))
                }
                // 以上 (>=)
                (NodeValue::Ge(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Bool(l >= r))
                }
                _ => Err("Unsupported operation or mismatched types in condition".to_string()),
            }
        }
        // 通常の演算子の処理
        else if let NodeValue::Add(left, right)
        | NodeValue::Sub(left, right)
        | NodeValue::Mul(left, right)
        | NodeValue::Div(left, right) = &node.node_value()
        {
            let left_value = self.evaluate(left)?;
            let right_value = self.evaluate(right)?;
            match (&node.node_value(), left_value, right_value) {
                (NodeValue::Add(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l + r))
                }
                (NodeValue::Sub(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l - r))
                }
                (NodeValue::Mul(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    Ok(VariableValue::Int32(l * r))
                }
                (NodeValue::Div(_, _), VariableValue::Int32(l), VariableValue::Int32(r)) => {
                    if r == 0 {
                        Err("Division by zero.".to_string())
                    } else {
                        Ok(VariableValue::Int32(l / r))
                    }
                }
                (NodeValue::Add(_, _), VariableValue::Str(l), VariableValue::Str(r)) => {
                    Ok(VariableValue::Str(l + &r))
                }
                _ => Err("Unsupported operation or mismatched types".to_string()),
            }
        } else {
            Err("Unsupported binary operation".to_string())
        }
    }
}
impl Decoder {
    pub fn new() -> Self {
        Decoder {
            global_variables: HashMap::new(),
            func_lists: HashMap::new(),
            comment_lists: HashMap::new(),
            last_var_name: None,
            current_node: None,
            current_function: None,
        }
    }
}
*/
