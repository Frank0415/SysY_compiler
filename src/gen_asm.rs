use crate::gen_asm_exp::*;
use crate::reg_alloc::{LinearScanAlloc, VariableLocation};
use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::{Function, FunctionData, Program, Value};
use std::collections::HashMap;
use std::fmt::Error;

fn fits_i12(x: isize) -> bool {
    (-2048..=2047).contains(&x)
}

fn emit_adjust_sp(delta: isize) -> String {
    if delta == 0 {
        return String::new();
    }
    if fits_i12(delta) {
        format!("\taddi sp, sp, {}\n", delta)
    } else {
        format!("\tli t0, {}\n\tadd sp, sp, t0\n", delta)
    }
}

fn emit_store_to_sp(src: &str, offset: usize) -> String {
    if fits_i12(offset as isize) {
        format!("\tsw {}, {}(sp)\n", src, offset)
    } else {
        format!(
            "\tli t0, {}\n\tadd t0, sp, t0\n\tsw {}, 0(t0)\n",
            offset, src
        )
    }
}

fn emit_load_from_sp(dst: &str, offset: usize) -> String {
    if fits_i12(offset as isize) {
        format!("\tlw {}, {}(sp)\n", dst, offset)
    } else {
        format!(
            "\tli t0, {}\n\tadd t0, sp, t0\n\tlw {}, 0(t0)\n",
            offset, dst
        )
    }
}

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
        global_names: &HashMap<Value, String>,
    ) -> String;
}

// 遍历函数
impl GenAsm for Program {
    fn gen_asm(&self) -> Result<String, Error> {
        let mut str = String::new();

        let mut global_str = String::new();
        for global_value in self.inst_layout() {
            let global_data = self.borrow_value(*global_value);
            let global_name = global_data
                .name()
                .as_ref()
                .expect("Global value should have a name");
            let global_name = &global_name[1..];

            let init = match global_data.kind() {
                ValueKind::GlobalAlloc(ga) => ga.init(),
                _ => continue,
            };

            global_str += &format!("\t.globl {}\n", global_name);
            global_str += &format!("{}:\n", global_name);
            emit_global_initializer(self, init, &mut global_str);
        }

        if !global_str.is_empty() {
            str += "\t.data\n";
            str += &global_str;
            str += "\n";
        }

        str += "\t.text\n";
        let mut func_names: HashMap<Function, String> = HashMap::new();
        let mut global_names: HashMap<Value, String> = HashMap::new();
        for &func in self.func_layout() {
            let name = &self.func(func).name()[1..];
            func_names.insert(func, name.to_string());
        }
        for global_value in self.inst_layout() {
            let global_data = self.borrow_value(*global_value);
            if let Some(name) = global_data.name().as_ref() {
                global_names.insert(*global_value, name[1..].to_string());
            }
        }
        for &func in self.func_layout() {
            if self.func(func).layout().entry_bb().is_none() {
                continue;
            }
            str += &gen_func_asm(self.func(func), &func_names, &global_names).unwrap();
        }
        Ok(str)
    }
}

fn emit_global_initializer(program: &Program, value: Value, out: &mut String) {
    let data = program.borrow_value(value);
    match data.kind() {
        ValueKind::ZeroInit(_) => {
            out.push_str(&format!("\t.zero {}\n", data.ty().size()));
        }
        ValueKind::Integer(int) => {
            out.push_str(&format!("\t.word {}\n", int.value()));
        }
        ValueKind::Aggregate(agg) => {
            for elem in agg.elems() {
                emit_global_initializer(program, *elem, out);
            }
        }
        _ => unimplemented!("Unsupported global initializer kind"),
    }
}

fn gen_func_asm(
    func_data: &FunctionData,
    func_names: &HashMap<Function, String>,
    global_names: &HashMap<Value, String>,
) -> Result<String, Error> {
    let name = &func_data.name()[1..];
    let mut str = String::new();
    str += &format!("\t.globl {}\n", name);
    str += &format!("{}:\n", name);
    let mut reg_alloc: LinearScanAlloc = LinearScanAlloc::new();
    reg_alloc.allocate(func_data);
    let stack_size = reg_alloc.get_stack_count();
    if stack_size > 0 {
        str += &emit_adjust_sp(-(stack_size as isize));
    }
    if let Some(ra_offset) = reg_alloc.get_ra_offset() {
        str += &emit_store_to_sp("ra", ra_offset);
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
                .translate_inst(&inst, func_data.dfg(), &reg_alloc, func_names, global_names);
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
        global_names: &HashMap<Value, String>,
    ) -> String {
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
                                res += &emit_load_from_sp("a0", stack_offset);
                            }
                            VariableLocation::None => {
                                unreachable!("Return value not found in register or stack");
                            }
                        }
                    }
                }
                let stack_size = reg_alloc.get_stack_count();
                if let Some(ra_offset) = reg_alloc.get_ra_offset() {
                    res += &emit_load_from_sp("ra", ra_offset);
                }
                if stack_size > 0 {
                    res += &emit_adjust_sp(stack_size as isize);
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
                    asm += &emit_store_to_sp(&target, offset);
                    reg_alloc.release_scratch(target);
                }
                asm
            }
            ValueKind::Alloc(_alloc) => process_alloc_inst(),
            ValueKind::Load(load) => process_load_inst(load, value, dfg, reg_alloc, global_names),
            ValueKind::Store(store) => process_store_inst(store, dfg, reg_alloc, global_names),
            ValueKind::GetElemPtr(getelemptr) => {
                process_getelemptr_inst(getelemptr, value, dfg, reg_alloc, global_names)
            }
            ValueKind::GetPtr(getptr) => {
                process_getptr_inst(getptr, value, dfg, reg_alloc, global_names)
            }
            ValueKind::Call(call) => process_call_inst(call, value, dfg, reg_alloc, func_names),
            ValueKind::Branch(branch) => process_branch_inst(branch, dfg, reg_alloc),
            ValueKind::Jump(jmp) => process_jump_inst(jmp, dfg, reg_alloc),
            _ => unimplemented!(),
        }
    }
}
