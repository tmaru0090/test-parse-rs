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
    pub fn push_stack_frame(&mut self, func_name: &str) {
        self.stack_frames
            .insert(func_name.to_string(), StackFrame { blocks: Vec::new() });
    }

    pub fn pop_stack_frame(&mut self, func_name: &str) {
        self.stack_frames.remove(func_name);
    }

    pub fn add_to_stack_frame(&mut self, func_name: &str, block: MemoryBlock) {
        if let Some(frame) = self.stack_frames.get_mut(func_name) {
            frame.blocks.push(block);
        }
    }
    // 指定の型の値を確保してUUIDのアドレスを返す
    pub fn allocate<T: 'static + Any>(&mut self, value: T) -> Uuid {
        let id = if let Some(free_id) = self.free_list.pop() {
            // 解放済みのブロックがあれば再利用
            free_id
        } else {
            Uuid::new_v4() // 新しいUUIDを生成
        };
        let block = MemoryBlock {
            id,
            value: Box::new(value),
        };
        self.heap.insert(id, block);
        id // 割り当てたメモリのIDを返す
    }
    // 指定アドレス(UUID)のメモリを開放
    pub fn deallocate(&mut self, id: Uuid) {
        if self.heap.remove(&id).is_some() {
            self.free_list.push(id); // 解放されたメモリブロックをフリーリストに追加
        }
    }
    // 指定のアドレスの値を返す
    pub fn get_value<T: 'static + Any>(&self, id: Uuid) -> Option<&T> {
        self.heap
            .get(&id)
            .and_then(|block| block.value.downcast_ref::<T>()) // IDから値を取得
    }
    // 指定のアドレスの値を更新
    pub fn update_value<T: 'static + Any>(&mut self, id: Uuid, new_value: T) -> bool {
        if let Some(block) = self.heap.get_mut(&id) {
            block.value = Box::new(new_value); // 新しい値で更新
            true
        } else {
            false // 指定されたIDが見つからなかった場合
        }
    }
}
