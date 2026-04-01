use crate::gen_asm_exp::*;
use crate::reg_alloc::{LinearScanAlloc, VariableLocation};
use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::{Function, FunctionData, Program, Value};
use std::fmt::Error;
use std::collections::HashMap;

pub trait GenAsm {
    fn gen_asm(&self) -> Result<String, Error>;
}

trait LocalGenAsm {
    fn translate_inst(
        &self,
        value: &Value,
        dfg: &DataFlowGraph,
        reg_alloc: &LinearScanAlloc,
        func_names: &HashMap<Function, String>,
    ) -> String;
}

// 遍历函数
impl GenAsm for Program {
    fn gen_asm(&self) -> Result<String, Error> {
        let mut str = String::new();
        str += "\t.text\n";
        let mut func_names: HashMap<Function, String> = HashMap::new();
        for &func in self.func_layout() {
            let name = &self.func(func).name()[1..];
            func_names.insert(func, name.to_string());
        }
        for &func in self.func_layout() {
            if self.func(func).layout().entry_bb().is_none() {
                continue;
            }
            str += &gen_func_asm(self.func(func), &func_names).unwrap();
        }
        Ok(str)
    }
}

fn gen_func_asm(
    func_data: &FunctionData,
    func_names: &HashMap<Function, String>,
) -> Result<String, Error> {
    let name = &func_data.name()[1..];
    let mut str = String::new();
    str += &format!("\t.globl {}\n", name);
    str += &format!("{}:\n", name);
    let mut reg_alloc: LinearScanAlloc = LinearScanAlloc::new();
    reg_alloc.allocate(func_data);
    let stack_size = reg_alloc.get_stack_count();
    if stack_size > 0 {
        str += &format!("\taddi sp, sp, -{}\n", stack_size);
    }
    if let Some(ra_offset) = reg_alloc.get_ra_offset() {
        str += &format!("\tsw ra, {}(sp)\n", ra_offset);
    }
    for (&bb, node) in func_data.layout().bbs() {
        // Add the label for the basic block (skip %entry which is handled by function name)
        let bb_name = &func_data.dfg().bb(bb).name().as_ref().unwrap()[1..];
        if bb_name != "entry" {
            str += &format!("{}:\n", bb_name);
        }
        for &inst in node.insts().keys() {
            str += &func_data
                .dfg()
                .value(inst)
                .translate_inst(&inst, func_data.dfg(), &reg_alloc, func_names);
        }
    }
    Ok(str)
}

impl LocalGenAsm for ValueData {
    fn translate_inst(
        &self,
        value: &Value,
        dfg: &DataFlowGraph,
        reg_alloc: &LinearScanAlloc,
        func_names: &HashMap<Function, String>,
    ) -> String {
        println!("Translating instruction: {:?}", self.kind());
        self.used_by();
        match self.kind() {
            ValueKind::Return(ret) => {
                let mut res = String::new();
                if let Some(v) = ret.value() {
                    if let ValueKind::Integer(int) = dfg.value(v).kind() {
                        res += &format!("\tli a0, {}\n", int.value());
                    } else {
                        match reg_alloc.get_variable(&v) {
                            VariableLocation::Register(reg) => {
                                res += &format!("\tmv a0, {}\n", reg);
                            }
                            VariableLocation::Stack(stack_offset) => {
                                res += &format!("\tlw a0, {}(sp)\n", stack_offset);
                            }
                            VariableLocation::None => {
                                unreachable!("Return value not found in register or stack");
                            }
                        }
                    }
                }
                let stack_size = reg_alloc.get_stack_count();
                if let Some(ra_offset) = reg_alloc.get_ra_offset() {
                    res += &format!("\tlw ra, {}(sp)\n", ra_offset);
                }
                if stack_size > 0 {
                    res += &format!("\taddi sp, sp, {}\n", stack_size);
                }
                res += "\tret\n";
                res
            }
            ValueKind::Binary(bin) => {
                let target_loc = reg_alloc.get_variable(value);
                let target = match &target_loc {
                    VariableLocation::Register(reg) => reg.clone(),
                    VariableLocation::Stack(_) => reg_alloc.acquire_scratch(),
                    VariableLocation::None => panic!("Binary result has no location"),
                };
                let mut asm = match bin.op() {
                    koopa::ir::BinaryOp::Add => process_inst(bin, dfg, reg_alloc, target.clone(), "add"),
                    koopa::ir::BinaryOp::Sub => process_inst(bin, dfg, reg_alloc, target.clone(), "sub"),
                    koopa::ir::BinaryOp::Mul => process_inst(bin, dfg, reg_alloc, target.clone(), "mul"),
                    koopa::ir::BinaryOp::Div => process_inst(bin, dfg, reg_alloc, target.clone(), "div"),
                    koopa::ir::BinaryOp::Mod => process_inst(bin, dfg, reg_alloc, target.clone(), "rem"),
                    koopa::ir::BinaryOp::Eq => process_eq_inst(bin, dfg, reg_alloc, target.clone()),
                    koopa::ir::BinaryOp::NotEq => process_neq_inst(bin, dfg, reg_alloc, target.clone()),
                    koopa::ir::BinaryOp::Lt => process_inst(bin, dfg, reg_alloc, target.clone(), "slt"),
                    koopa::ir::BinaryOp::Gt => process_inst(bin, dfg, reg_alloc, target.clone(), "sgt"),
                    koopa::ir::BinaryOp::Le => process_le_inst(bin, dfg, reg_alloc, target.clone()),
                    koopa::ir::BinaryOp::Ge => process_ge_inst(bin, dfg, reg_alloc, target.clone()),
                    koopa::ir::BinaryOp::And => process_and_inst(bin, dfg, reg_alloc, target.clone()),
                    koopa::ir::BinaryOp::Or => process_or_inst(bin, dfg, reg_alloc, target.clone()),
                    _ => {
                        println!(
                            "placeholder for binary operation: {:?} op#1: {:?}, op#2: {:?}\n",
                            bin.op(),
                            bin.lhs(),
                            bin.rhs()
                        );
                        unimplemented!();
                    }
                };
                if let VariableLocation::Stack(offset) = target_loc {
                    asm += &format!("\tsw {}, {}(sp)\n", target, offset);
                    reg_alloc.release_scratch(target);
                }
                asm
            }
            ValueKind::Alloc(_alloc) => process_alloc_inst(),
            ValueKind::Load(load) => process_load_inst(load, value, dfg, reg_alloc),
            ValueKind::Store(store) => process_store_inst(store, dfg, reg_alloc),
            ValueKind::Call(call) => process_call_inst(call, value, dfg, reg_alloc, func_names),
            ValueKind::Branch(branch) => process_branch_inst(branch, dfg, reg_alloc),
            ValueKind::Jump(jmp) => process_jump_inst(jmp, dfg, reg_alloc),
            _ => unimplemented!(),
        }
    }
}
