
use std::any::Any;
use std::collections::HashMap;
use uuid::Uuid;
impl Clone for MemoryBlock {
    fn clone(&self) -> Self {
        // クローン処理。今回はidのみクローンし、valueはクローン不可のため新たに初期化
        MemoryBlock {
            id: self.id,
            value: Box::new(()), // クローンできないためデフォルトの空の値を持たせる
        }
    }
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
#[derive(Debug)]
pub struct MemoryBlock {
    pub id: Uuid,
    pub value: Box<dyn Any>,
}

// メモリの管理
#[derive(Debug, Clone)]
pub struct MemoryManager {
    pub heap: HashMap<Uuid, MemoryBlock>, // ヒープ(アドレス,値)
    pub free_list: Vec<Uuid>,
    pub stack_frames: HashMap<String, StackFrame>, // 関数名ごとのスタックフレーム
}
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub blocks: Vec<MemoryBlock>, // スタックフレーム内のメモリブロック
}
impl MemoryManager {
    pub fn new(heap_size: usize) -> Self {
        MemoryManager {
            heap: HashMap::new(),
            free_list: Vec::new(),
            stack_frames: HashMap::new(),
        }
    }
}

