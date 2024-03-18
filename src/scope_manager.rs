use crate::types::*;
use std::collections::HashMap;

pub struct ScopeManager {
    pub scopes: Vec<HashMap<String, VarType>>, // スコープ毎の変数名と値の関連付けを管理するVec
}

impl ScopeManager {
    pub fn new() -> ScopeManager {
        ScopeManager {
            scopes: vec![HashMap::new()],
        } // 初期スコープを作成
    }

    pub fn create_scope(&mut self) {
        self.scopes.push(HashMap::new()); // 新しいスコープを作成して追加
    }

    pub fn destroy_scope(&mut self) {
        self.scopes.pop(); // 最後のスコープを削除
    }

    pub fn set_variable(&mut self, name: String, value: VarType) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value); // 現在のスコープに変数を追加
            Ok(())
        } else {
            Err("No scope exists".to_string())
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<VarType> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone()); // スコープから変数を取得
            }
        }
        None
    }
}
