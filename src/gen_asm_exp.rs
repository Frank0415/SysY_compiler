use crate::reg_alloc::{LinearScanAlloc, VariableLocation};
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::values::{Binary, Branch, Call, GetElemPtr, GetPtr, Jump, Load, Store};
use koopa::ir::{Function, TypeKind, Value, ValueKind};
use std::collections::HashMap;

struct LoadedBinaryOperands {
    asm: String,
    leftreg: String,
    rightreg: String,
    borrowed_scratch: Vec<String>,
}

fn release_scratch_regs(reg_alloc: &LinearScanAlloc, regs: Vec<String>) {
    for reg in regs {
        reg_alloc.release_scratch(reg);
    }
}

fn fits_i12(x: isize) -> bool {
    (-2048..=2047).contains(&x)
}

fn emit_load_from_sp(dst: &str, offset: usize, reg_alloc: &LinearScanAlloc) -> String {
    if fits_i12(offset as isize) {
        format!("\tlw {}, {}(sp)\n", dst, offset)
    } else {
        let addr = reg_alloc.acquire_scratch();
        let s = format!(
            "\tli {}, {}\n\tadd {}, sp, {}\n\tlw {}, 0({})\n",
            addr, offset, addr, addr, dst, addr
        );
        reg_alloc.release_scratch(addr);
        s
    }
}

fn emit_store_to_sp(src: &str, offset: usize, reg_alloc: &LinearScanAlloc) -> String {
    if fits_i12(offset as isize) {
        format!("\tsw {}, {}(sp)\n", src, offset)
    } else {
        let addr = reg_alloc.acquire_scratch();
        let s = format!(
            "\tli {}, {}\n\tadd {}, sp, {}\n\tsw {}, 0({})\n",
            addr, offset, addr, addr, src, addr
        );
        reg_alloc.release_scratch(addr);
        s
    }
}

fn emit_addr_from_sp(dst: &str, offset: usize) -> String {
    if fits_i12(offset as isize) {
        format!("\taddi {}, sp, {}\n", dst, offset)
    } else {
        format!("\tli {}, {}\n\tadd {}, sp, {}\n", dst, offset, dst, dst)
    }
}

fn load_stack_to_scratch(offset: usize, scratch: &str) -> String {
    if fits_i12(offset as isize) {
        format!("\tlw {}, {}(sp)\n", scratch, offset)
    } else {
        format!(
            "\tli {}, {}\n\tadd {}, sp, {}\n\tlw {}, 0({})\n",
            scratch, offset, scratch, scratch, scratch, scratch
        )
    }
}

fn load_variable_stack_to_scratch(
    value: &Value,
    reg_alloc: &LinearScanAlloc,
    scratch: &str,
) -> String {
    match reg_alloc.get_variable(value) {
        VariableLocation::Stack(offset) => load_stack_to_scratch(offset, scratch),
        _ => panic!("Value {:?} is not in stack", value),
    }
}

fn materialize_operand(
    value: Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    preferred_scratch: &str,
) -> (String, String, bool) {
    if let ValueKind::Integer(int) = dfg.value(value).kind() {
        let v = int.value();
        if v == 0 {
            (String::new(), "x0".to_string(), false)
        } else {
            (
                format!("\tli\t{}, {}\n", preferred_scratch, v),
                preferred_scratch.to_string(),
                true,
            )
        }
    } else {
        match reg_alloc.get_variable(&value) {
            VariableLocation::Register(reg) => (String::new(), reg, false),
            VariableLocation::Stack(offset) => (
                load_stack_to_scratch(offset, preferred_scratch),
                preferred_scratch.to_string(),
                true,
            ),
            VariableLocation::None => panic!("Operand {:?} has no location", value),
        }
    }
}

// 先尝试使用教程的汇编，而不是使用更简便的形式
pub fn process_eq_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let loaded = load_temp_int(bin, dfg, reg_alloc);
    asm += &loaded.asm;
    let leftreg = loaded.leftreg;
    let rightreg = loaded.rightreg;
    asm += &format!("\txor\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    release_scratch_regs(reg_alloc, loaded.borrowed_scratch);
    asm
}

pub fn process_neq_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let loaded = load_temp_int(bin, dfg, reg_alloc);
    asm += &loaded.asm;
    let leftreg = loaded.leftreg;
    let rightreg = loaded.rightreg;
    asm += &format!("\txor\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tsnez\t{}, {}\n", target, target);
    release_scratch_regs(reg_alloc, loaded.borrowed_scratch);
    asm
}

pub fn process_le_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let loaded = load_temp_int(bin, dfg, reg_alloc);
    asm += &loaded.asm;
    let leftreg = loaded.leftreg;
    let rightreg = loaded.rightreg;
    // a <= b is !(a > b) -> !(sgt target, left, right)
    asm += &format!("\tsgt\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    release_scratch_regs(reg_alloc, loaded.borrowed_scratch);
    asm
}

pub fn process_ge_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let loaded = load_temp_int(bin, dfg, reg_alloc);
    asm += &loaded.asm;
    let leftreg = loaded.leftreg;
    let rightreg = loaded.rightreg;
    // a >= b is !(a < b) -> !(slt target, left, right)
    asm += &format!("\tslt\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    release_scratch_regs(reg_alloc, loaded.borrowed_scratch);
    asm
}

pub fn process_and_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let left_scratch = reg_alloc.acquire_scratch();
    let right_scratch = reg_alloc.acquire_scratch();
    let (left_asm, leftreg, _) = materialize_operand(bin.lhs(), dfg, reg_alloc, &left_scratch);
    let (right_asm, rightreg, _) = materialize_operand(bin.rhs(), dfg, reg_alloc, &right_scratch);
    asm += &left_asm;
    asm += &right_asm;
    if leftreg != left_scratch {
        asm += &format!("\tmv\t{}, {}\n", left_scratch, leftreg);
    }
    if rightreg != right_scratch {
        asm += &format!("\tmv\t{}, {}\n", right_scratch, rightreg);
    }
    // Logical AND: (lhs != 0) && (rhs != 0)
    asm += &format!("\tsnez\t{}, {}\n", left_scratch, left_scratch);
    asm += &format!("\tsnez\t{}, {}\n", right_scratch, right_scratch);
    asm += &format!("\tand\t{}, {}, {}\n", target, left_scratch, right_scratch);
    reg_alloc.release_scratch(left_scratch);
    reg_alloc.release_scratch(right_scratch);
    asm
}

pub fn process_or_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm = String::new();
    let left_scratch = reg_alloc.acquire_scratch();
    let right_scratch = reg_alloc.acquire_scratch();
    let (left_asm, leftreg, _) = materialize_operand(bin.lhs(), dfg, reg_alloc, &left_scratch);
    let (right_asm, rightreg, _) = materialize_operand(bin.rhs(), dfg, reg_alloc, &right_scratch);
    asm += &left_asm;
    asm += &right_asm;
    if leftreg != left_scratch {
        asm += &format!("\tmv\t{}, {}\n", left_scratch, leftreg);
    }
    if rightreg != right_scratch {
        asm += &format!("\tmv\t{}, {}\n", right_scratch, rightreg);
    }
    // Logical OR: (lhs != 0) || (rhs != 0)
    asm += &format!("\tsnez\t{}, {}\n", left_scratch, left_scratch);
    asm += &format!("\tsnez\t{}, {}\n", right_scratch, right_scratch);
    asm += &format!("\tor\t{}, {}, {}\n", target, left_scratch, right_scratch);
    reg_alloc.release_scratch(left_scratch);
    reg_alloc.release_scratch(right_scratch);
    asm
}

pub fn process_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
    inst: &str,
) -> String {
    let mut asm = String::new();
    let loaded = load_temp_int(bin, dfg, reg_alloc);
    asm += &loaded.asm;
    let leftreg = loaded.leftreg;
    let rightreg = loaded.rightreg;

    asm += &format!("\t{}\t{}, {}, {}\n", inst, target, leftreg, rightreg);
    release_scratch_regs(reg_alloc, loaded.borrowed_scratch);
    asm
}

pub fn process_call_inst(
    call: &Call,
    inst: &koopa::ir::Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    func_names: &HashMap<Function, String>,
) -> String {
    let mut ret = String::new();

    for (idx, arg) in call.args().iter().enumerate() {
        if idx < 8 {
            let a_reg = format!("a{}", idx);
            if let ValueKind::Integer(int) = dfg.value(*arg).kind() {
                ret += &format!("\tli {}, {}\n", a_reg, int.value());
            } else {
                match reg_alloc.get_variable(arg) {
                    VariableLocation::Register(reg) => {
                        ret += &format!("\tmv {}, {}\n", a_reg, reg);
                    }
                    VariableLocation::Stack(offset) => {
                        ret += &emit_load_from_sp(&a_reg, offset, reg_alloc);
                    }
                    VariableLocation::None => panic!("Call arg has no location: {:?}", arg),
                }
            }
        } else {
            let stack_off = (idx - 8) * 4;
            if let ValueKind::Integer(int) = dfg.value(*arg).kind() {
                let scratch = reg_alloc.acquire_scratch();
                ret += &format!("\tli {}, {}\n", scratch, int.value());
                ret += &emit_store_to_sp(&scratch, stack_off, reg_alloc);
                reg_alloc.release_scratch(scratch);
            } else {
                match reg_alloc.get_variable(arg) {
                    VariableLocation::Register(reg) => {
                        ret += &emit_store_to_sp(&reg, stack_off, reg_alloc);
                    }
                    VariableLocation::Stack(offset) => {
                        let scratch = reg_alloc.acquire_scratch();
                        ret += &emit_load_from_sp(&scratch, offset, reg_alloc);
                        ret += &emit_store_to_sp(&scratch, stack_off, reg_alloc);
                        reg_alloc.release_scratch(scratch);
                    }
                    VariableLocation::None => panic!("Call arg has no location: {:?}", arg),
                }
            }
        }
    }

    let callee_name = func_names
        .get(&call.callee())
        .expect("Call callee function name should exist");
    ret += &format!("\tcall {}\n", callee_name);

    match reg_alloc.get_variable(inst) {
        VariableLocation::Register(reg) => {
            ret += &format!("\tmv {}, a0\n", reg);
        }
        VariableLocation::Stack(offset) => {
            ret += &emit_store_to_sp("a0", offset, reg_alloc);
        }
        VariableLocation::None => {}
    }

    ret
}

fn load_temp_int(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> LoadedBinaryOperands {
    let op1 = bin.lhs();
    let op2 = bin.rhs();

    let left_scratch = reg_alloc.acquire_scratch();
    let right_scratch = reg_alloc.acquire_scratch();
    let (left_asm, leftreg, left_uses_scratch) =
        materialize_operand(op1, dfg, reg_alloc, &left_scratch);
    let (right_asm, rightreg, right_uses_scratch) =
        materialize_operand(op2, dfg, reg_alloc, &right_scratch);

    let mut borrowed_scratch = Vec::new();
    if left_uses_scratch {
        borrowed_scratch.push(left_scratch.clone());
    } else {
        reg_alloc.release_scratch(left_scratch);
    }
    if right_uses_scratch {
        borrowed_scratch.push(right_scratch.clone());
    } else {
        reg_alloc.release_scratch(right_scratch);
    }

    LoadedBinaryOperands {
        asm: format!("{}{}", left_asm, right_asm),
        leftreg,
        rightreg,
        borrowed_scratch,
    }
}

pub fn process_alloc_inst() -> String {
    String::new()
}

fn load_pointer_base(
    src: Value,
    base_reg: &str,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    match reg_alloc.get_variable(&src) {
        VariableLocation::Register(reg) => format!("\tmv {}, {}\n", base_reg, reg),
        VariableLocation::Stack(offset) => {
            if matches!(dfg.value(src).kind(), ValueKind::Alloc(_)) {
                emit_addr_from_sp(base_reg, offset)
            } else {
                emit_load_from_sp(base_reg, offset, reg_alloc)
            }
        }
        VariableLocation::None => {
            let global_name = global_names
                .get(&src)
                .unwrap_or_else(|| panic!("Unsupported pointer source"));
            format!("\tla {}, {}\n", base_reg, global_name)
        }
    }
}

fn lower_ptr_calc(
    src: Value,
    index: Value,
    inst: &koopa::ir::Value,
    elem_size: usize,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    let mut ret = String::new();

    let base = reg_alloc.acquire_scratch();
    let idx = reg_alloc.acquire_scratch();
    let scale = reg_alloc.acquire_scratch();

    ret += &load_pointer_base(src, &base, dfg, reg_alloc, global_names);

    if let ValueKind::Integer(int) = dfg.value(index).kind() {
        ret += &format!("\tli {}, {}\n", idx, int.value());
    } else {
        match reg_alloc.get_variable(&index) {
            VariableLocation::Register(reg) => ret += &format!("\tmv {}, {}\n", idx, reg),
            VariableLocation::Stack(offset) => ret += &emit_load_from_sp(&idx, offset, reg_alloc),
            VariableLocation::None => panic!("Index has no location"),
        }
    }

    ret += &format!("\tli {}, {}\n", scale, elem_size);
    ret += &format!("\tmul {}, {}, {}\n", idx, idx, scale);
    ret += &format!("\tadd {}, {}, {}\n", base, base, idx);

    match reg_alloc.get_variable(inst) {
        VariableLocation::Register(reg) => ret += &format!("\tmv {}, {}\n", reg, base),
        VariableLocation::Stack(offset) => ret += &emit_store_to_sp(&base, offset, reg_alloc),
        VariableLocation::None => panic!("Pointer result has no location"),
    }

    reg_alloc.release_scratch(scale);
    reg_alloc.release_scratch(idx);
    reg_alloc.release_scratch(base);

    ret
}

pub fn process_getelemptr_inst(
    gep: &GetElemPtr,
    inst: &koopa::ir::Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    let elem_size = match dfg.value(*inst).ty().kind() {
        TypeKind::Pointer(base) => base.size(),
        _ => panic!("GetElemPtr result must be a pointer"),
    };
    lower_ptr_calc(
        gep.src(),
        gep.index(),
        inst,
        elem_size,
        dfg,
        reg_alloc,
        global_names,
    )
}

pub fn process_getptr_inst(
    gp: &GetPtr,
    inst: &koopa::ir::Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    let elem_size = match dfg.value(*inst).ty().kind() {
        TypeKind::Pointer(base) => base.size(),
        _ => panic!("GetPtr result must be a pointer"),
    };
    lower_ptr_calc(
        gp.src(),
        gp.index(),
        inst,
        elem_size,
        dfg,
        reg_alloc,
        global_names,
    )
}

pub fn process_load_inst(
    load: &Load,
    inst: &koopa::ir::Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    let mut ret = String::new();
    let src = load.src();

    let val_tmp = reg_alloc.acquire_scratch();
    match dfg.value(src).kind() {
        ValueKind::Alloc(_) => {
            let src_offset = match reg_alloc.get_variable(&src) {
                VariableLocation::Stack(offset) => offset,
                _ => panic!("Alloc source should be stack allocated"),
            };
            ret += &emit_load_from_sp(&val_tmp, src_offset, reg_alloc);
        }
        _ => {
            let addr = reg_alloc.acquire_scratch();
            ret += &load_pointer_base(src, &addr, dfg, reg_alloc, global_names);
            ret += &format!("\tlw {}, 0({})\n", val_tmp, addr);
            reg_alloc.release_scratch(addr);
        }
    }

    match reg_alloc.get_variable(inst) {
        VariableLocation::Register(reg) => ret += &format!("\tmv {}, {}\n", reg, val_tmp),
        VariableLocation::Stack(stack) => ret += &emit_store_to_sp(&val_tmp, stack, reg_alloc),
        VariableLocation::None => panic!("Load destination has neither register nor stack slot"),
    }
    reg_alloc.release_scratch(val_tmp);

    ret
}

pub fn process_store_inst(
    store: &Store,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    global_names: &HashMap<Value, String>,
) -> String {
    let mut ret = String::new();
    let val = store.value();
    let dest = store.dest();
    let val_reg = reg_alloc.acquire_scratch();

    if let koopa::ir::ValueKind::Integer(int) = dfg.value(val).kind() {
        let v = int.value();
        if v == 0 {
            ret += &format!("\tmv {}, x0\n", val_reg);
        } else {
            ret += &format!("\tli {}, {}\n", val_reg, v);
        }
    } else {
        match dfg.value(val).kind() {
            ValueKind::FuncArgRef(arg_ref) => {
                let idx = arg_ref.index();
                if idx < 8 {
                    ret += &format!("\tmv {}, a{}\n", val_reg, idx);
                } else {
                    let caller_arg_offset = reg_alloc.get_stack_count() + (idx - 8) * 4;
                    ret += &emit_load_from_sp(&val_reg, caller_arg_offset, reg_alloc);
                }
            }
            _ => match reg_alloc.get_variable(&val) {
                VariableLocation::Register(reg) => {
                    ret += &format!("\tmv {}, {}\n", val_reg, reg);
                }
                VariableLocation::Stack(offset) => {
                    ret += &emit_load_from_sp(&val_reg, offset, reg_alloc);
                }
                VariableLocation::None => panic!("Store value has neither register nor stack slot"),
            },
        }
    }

    match dfg.value(dest).kind() {
        ValueKind::Alloc(_) => {
            let dest_offset = match reg_alloc.get_variable(&dest) {
                VariableLocation::Stack(offset) => offset,
                _ => panic!("Alloc destination should be stack allocated"),
            };
            ret += &emit_store_to_sp(&val_reg, dest_offset, reg_alloc);
        }
        _ => {
            let addr = reg_alloc.acquire_scratch();
            ret += &load_pointer_base(dest, &addr, dfg, reg_alloc, global_names);
            ret += &format!("\tsw {}, 0({})\n", val_reg, addr);
            reg_alloc.release_scratch(addr);
        }
    }

    reg_alloc.release_scratch(val_reg);
    ret
}

pub fn process_branch_inst(
    branch: &Branch,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> String {
    let mut ret = String::new();
    let cond = branch.cond();
    let then_bb = branch.true_bb();
    let else_bb = branch.false_bb();
    let then_name = &dfg.bb(then_bb).name().as_ref().unwrap()[1..];
    let else_name = &dfg.bb(else_bb).name().as_ref().unwrap()[1..];

    let scratch = reg_alloc.acquire_scratch();
    let (cond_asm, cond_reg, _) = materialize_operand(cond, dfg, reg_alloc, &scratch);
    ret += &cond_asm;

    // 2. Generate branch: if cond != 0 jump to then_name, else jump to else_name
    ret += &format!("\tbnez {}, {}\n", cond_reg, then_name);
    ret += &format!("\tj {}\n", else_name);
    reg_alloc.release_scratch(scratch);

    ret
}

pub fn process_jump_inst(jump: &Jump, dfg: &DataFlowGraph, _reg_alloc: &LinearScanAlloc) -> String {
    let jump_target = &dfg.bb(jump.target()).name().as_ref().unwrap()[1..];
    format!("\tj {}\n", jump_target)
}
