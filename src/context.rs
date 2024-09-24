use indexmap::IndexMap;
use serde_json::Value;
use uuid::Uuid;
// 変数情報
#[derive(Debug, Clone)]
pub struct Variable {
    pub data_type: Value, // 型
    pub value: Value,     // 値
    pub address: Uuid,    // アドレス
    pub is_mutable: bool, // 可変性
    pub size: usize,      // サイズ
}

// コンテキスト
#[derive(Debug, Clone)]
pub struct Context {
    pub local_context: IndexMap<String, Variable>, // ローカルスコープ
    pub global_context: IndexMap<String, Variable>, // グローバルスコープ
    pub type_context: IndexMap<String, String>,    // グローバルス型定義スコープ
    pub comment_lists: IndexMap<(usize, usize), Vec<String>>, // コメントリスト
    pub used_context: IndexMap<String, (usize, usize, bool)>, // 参照カウント(変数名,(行数,列数,参照カウント))
}
impl Context {
    pub fn new() -> Self {
        Context {
            local_context: IndexMap::new(),
            global_context: IndexMap::new(),
            type_context: IndexMap::new(),
            comment_lists: IndexMap::new(),
            used_context: IndexMap::new(),
        }
    }
}
