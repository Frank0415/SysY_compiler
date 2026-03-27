use koopa::ir::{BasicBlock, Value};
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
    // Stack of while scopes
    // To record what block continue or break could jump to
    jump_stack: Context,
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
            jump_stack: Context::new(),
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

    pub fn enter_while(&mut self, entry_bb: &BasicBlock, end_bb: &BasicBlock) {
        self.jump_stack.enter_while(entry_bb, end_bb);
    }
    pub fn exit_while(&mut self) {
        self.jump_stack.exit_while();
    }
    pub fn get_continue(&self) -> Option<BasicBlock> {
        self.jump_stack.get_continue()
    }

    pub fn get_break(&mut self) -> Option<BasicBlock> {
        self.jump_stack.get_break()
    }
}

struct BaseContext {
    entry_bb: BasicBlock, // continue 跳转的目标（条件检查块）
    end_bb: BasicBlock, // break 跳转的目标（循环结束后的块）
}

pub struct Context {
    ctx: Vec<BaseContext>,
}

impl Context {
    fn new() -> Self {
        Context { ctx: Vec::new() }
    }

    fn enter_while(&mut self, entry_bb: &BasicBlock, end_bb: &BasicBlock) {
        self.ctx.push(BaseContext {
            entry_bb: *entry_bb,
            end_bb: *end_bb,
        });
    }

    fn exit_while(&mut self) {
        self.ctx.pop();
    }

    fn get_continue(&self) -> Option<BasicBlock> {
        if self.ctx.is_empty() {
            None
        } else {
            Some(self.ctx.last().unwrap().entry_bb)
        }
    }

    fn get_break(&self) -> Option<BasicBlock> {
        if self.ctx.is_empty() {
            None
        } else {
            Some(self.ctx.last().unwrap().end_bb)
        }
    }
}