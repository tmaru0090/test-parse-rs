use crate::tokenizer::*;
use crate::types::*;
#[derive(PartialEq, Debug, Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub value: String,
    pub child: Vec<Box<Node>>,
}
impl Node {
    pub fn new(node_type: NodeType, child: Vec<Box<Node>>, value: String) -> Node {
        Node {
            node_type,
            value,
            child,
        }
    }
}
