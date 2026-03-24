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
    // Global counter for unique labels
    counter: u64,
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}

impl Variables {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            counter: 0,
        }
    }

    pub fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
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

    pub fn contains_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .and_then(|scope| scope.get(name))
            .is_some()
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

    pub fn get_scope_layer(&self) -> usize {
        self.scopes.len()
    }
}
