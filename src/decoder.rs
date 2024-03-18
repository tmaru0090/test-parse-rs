use crate::node::*;
use crate::parser::Parser;
use crate::scope_manager::*;
use crate::types::*;
pub struct Decoder<'a> {
    pub parser: &'a Parser<'a>,
    pub scope_manager: &'a mut ScopeManager,
}

impl<'a> Decoder<'a> {
    pub fn new(parser: &'a Parser, scope_manager: &'a mut ScopeManager) -> Decoder<'a> {
        Decoder {
            parser,
            scope_manager,
        }
    }
    pub fn decode(&mut self, program: &Vec<Box<Node>>) -> Result<(), String> {
        for node in program.iter() {
            match &node.node_type {
                NodeType::VarDecl => {
                    if let Some(var_node) = node.child.get(0) {
                        if let NodeType::Var(name, val) = &var_node.node_type {
                            let var_name = name.clone();
                            let result = self.parser.eval(var_node)?;
                            self.scope_manager.set_variable(var_name.clone(), result)?;
                        }
                    }
                }
                _ => (),
            }
        }

        // 変数の表示は不要かもしれませんが、必要な場合は以下の行をコメントアウト解除してください
        for (index, node) in program.iter().enumerate() {
            self.parser.print_var(node, index)?;
        }

        Ok(())
    }
}
