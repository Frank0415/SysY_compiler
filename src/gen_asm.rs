use crate::gen_asm_exp::*;
use crate::reg_alloc::LinearScanAlloc;
use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::{FunctionData, Program, Value};
use std::fmt::Error;

pub trait GenAsm {
    fn gen_asm(&self) -> Result<String, Error>;
}

trait LocalGenAsm {
    fn translate_inst(
        &self,
        value: &Value,
        dfg: &DataFlowGraph,
        reg_alloc: &LinearScanAlloc,
    ) -> String;
}

// 遍历函数
impl GenAsm for Program {
    fn gen_asm(&self) -> Result<String, Error> {
        let mut str = String::new();
        str += "\t.text\n";
        for &func in self.func_layout() {
            str += &self.func(func).gen_asm().unwrap();
        }
        Ok(str)
    }
}

// 遍历基本块
impl GenAsm for FunctionData {
    fn gen_asm(&self) -> Result<String, Error> {
        let name = &self.name()[1..];
        let mut str = String::new();
        str += &format!("\t.globl {}\n", name);
        str += &format!("{}:\n", name);
        let mut reg_alloc: LinearScanAlloc = LinearScanAlloc::new();
        reg_alloc.allocate(self);
        let stack_size = reg_alloc.get_stack_count();
        if stack_size > 0 {
            str += &format!("\taddi sp, sp, -{}\n", stack_size);
        }
        for (&bb, node) in self.layout().bbs() {
            // Add the label for the basic block (skip %entry which is handled by function name)
            let bb_name = &self.dfg().bb(bb).name().as_ref().unwrap()[1..];
            if bb_name != "entry" {
                str += &format!("{}:\n", bb_name);
            }
            for &inst in node.insts().keys() {
                str += &self
                    .dfg()
                    .value(inst)
                    .translate_inst(&inst, self.dfg(), &reg_alloc);
            }
        }
        Ok(str)
    }
}

impl LocalGenAsm for ValueData {
    fn translate_inst(
        &self,
        value: &Value,
        dfg: &DataFlowGraph,
        reg_alloc: &LinearScanAlloc,
    ) -> String {
        println!("Translating instruction: {:?}", self.kind());
        self.used_by();
        match self.kind() {
            ValueKind::Return(ret) => {
                let mut res = String::new();
                if let Some(v) = ret.value() {
                    if let ValueKind::Integer(int) = dfg.value(v).kind() {
                        res += &format!("\tli a0, {}\n", int.value());
                    } else if let Some(reg) = reg_alloc.get_reg(&v) {
                        res += &format!("\tmv a0, {}\n", reg);
                    } else if let Some(stack_offset) = reg_alloc.get_stack(&v) {
                        res += &format!("\tlw a0, {}(sp)\n", stack_offset);
                    } else {
                        unreachable!("Return value not found in register or stack");
                    }
                }
                let stack_size = reg_alloc.get_stack_count();
                if stack_size > 0 {
                    res += &format!("\taddi sp, sp, {}\n", stack_size);
                }
                res += "\tret\n";
                res
            }
            ValueKind::Binary(bin) => {
                let target = reg_alloc
                    .get_reg(value)
                    .expect("Please implement stack regs logic!")
                    .clone();
                match bin.op() {
                    koopa::ir::BinaryOp::Add => process_inst(bin, dfg, reg_alloc, target, "add"),
                    koopa::ir::BinaryOp::Sub => process_inst(bin, dfg, reg_alloc, target, "sub"),
                    koopa::ir::BinaryOp::Mul => process_inst(bin, dfg, reg_alloc, target, "mul"),
                    koopa::ir::BinaryOp::Div => process_inst(bin, dfg, reg_alloc, target, "div"),
                    koopa::ir::BinaryOp::Mod => process_inst(bin, dfg, reg_alloc, target, "rem"),
                    koopa::ir::BinaryOp::Eq => process_eq_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::NotEq => process_neq_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Lt => process_inst(bin, dfg, reg_alloc, target, "slt"),
                    koopa::ir::BinaryOp::Gt => process_inst(bin, dfg, reg_alloc, target, "sgt"),
                    koopa::ir::BinaryOp::Le => process_le_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Ge => process_ge_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::And => process_and_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Or => process_or_inst(bin, dfg, reg_alloc, target),
                    _ => {
                        println!(
                            "placeholder for binary operation: {:?} op#1: {:?}, op#2: {:?}\n",
                            bin.op(),
                            bin.lhs(),
                            bin.rhs()
                        );
                        unimplemented!();
                    }
                }
            }
            ValueKind::Alloc(_alloc) => process_alloc_inst(),
            ValueKind::Load(load) => process_load_inst(load, value, dfg, reg_alloc),
            ValueKind::Store(store) => process_store_inst(store, dfg, reg_alloc),
            ValueKind::Branch(branch) => process_branch_inst(branch, dfg, reg_alloc),
            ValueKind::Jump(jmp) => process_jump_inst(jmp, dfg, reg_alloc),
            _ => unimplemented!(),
        }
    }
}
