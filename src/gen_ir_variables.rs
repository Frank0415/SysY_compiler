use koopa::ir::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]

pub enum SymbolInfo {
    Const(i32),
    Var(Value),
    None,
}

pub struct Variables {
    // Stack of scopes, where each scope is a Map
    scopes: Vec<HashMap<String, SymbolInfo>>,
}

impl Variables {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn insert(&mut self, name: String, value: SymbolInfo) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert(name, value);
        }
    }

    pub fn get_const(&self, name: &str) -> Option<i32> {
        // Search from most recent scope to oldest
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| match scope.get(name) {
                Some(SymbolInfo::Const(val)) => Some(*val),
                _ => None,
            })
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| match scope.get(name) {
                Some(SymbolInfo::Var(val)) => Some(*val),
                _ => None,
            })
    }
}
