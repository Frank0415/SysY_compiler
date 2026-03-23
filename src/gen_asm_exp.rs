use crate::reg_alloc::{LinearScanAlloc, VariableLocation};
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::values::{Binary, Branch, Jump, Load, Store};
use koopa::ir::{Value, ValueKind};

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

fn load_stack_to_scratch(offset: usize, scratch: &str) -> String {
    format!("\tlw {}, {}(sp)\n", scratch, offset)
}

fn load_variable_stack_to_scratch(value: &Value, reg_alloc: &LinearScanAlloc, scratch: &str) -> String {
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

pub fn process_load_inst(
    load: &Load,
    inst: &koopa::ir::Value,
    _dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> String {
    let mut ret = String::new();
    let src = load.src();
    let src_offset = match reg_alloc.get_variable(&src) {
        VariableLocation::Stack(offset) => offset,
        _ => panic!("Source not found on stack"),
    };
    match reg_alloc.get_variable(inst) {
        VariableLocation::Register(reg) => {
            ret += &format!("\tlw {}, {}(sp)\n", reg, src_offset);
        }
        VariableLocation::Stack(stack) => {
            let scratch = reg_alloc.acquire_scratch();
            ret += &load_stack_to_scratch(src_offset, &scratch);
            ret += &format!("\tsw {}, {}(sp)\n", scratch, stack);
            reg_alloc.release_scratch(scratch);
        }
        VariableLocation::None => panic!("Load destination has neither register nor stack slot"),
    }
    ret
}

pub fn process_store_inst(
    store: &Store,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> String {
    let mut ret = String::new();
    let val = store.value();
    let dest = store.dest();
    let dest_offset = match reg_alloc.get_variable(&dest) {
        VariableLocation::Stack(offset) => offset,
        _ => panic!("Dest not found on stack"),
    };

    if let koopa::ir::ValueKind::Integer(int) = dfg.value(val).kind() {
        let v = int.value();
        if v == 0 {
            ret += &format!("\tsw x0, {}(sp)\n", dest_offset);
        } else {
            let scratch = reg_alloc.acquire_scratch();
            ret += &format!("\tli {}, {}\n", scratch, v);
            ret += &format!("\tsw {}, {}(sp)\n", scratch, dest_offset);
            reg_alloc.release_scratch(scratch);
        }
    } else {
        match reg_alloc.get_variable(&val) {
            VariableLocation::Register(reg) => {
                ret += &format!("\tsw {}, {}(sp)\n", reg, dest_offset);
            }
            VariableLocation::Stack(_) => {
                let scratch = reg_alloc.acquire_scratch();
                ret += &load_variable_stack_to_scratch(&val, reg_alloc, &scratch);
                ret += &format!("\tsw {}, {}(sp)\n", scratch, dest_offset);
                reg_alloc.release_scratch(scratch);
            }
            VariableLocation::None => panic!("Store value has neither register nor stack slot"),
        }
    }

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

pub fn process_jump_inst(
    jump: &Jump,
    dfg: &DataFlowGraph,
    _reg_alloc: &LinearScanAlloc,
) -> String {
    let jump_target = &dfg.bb(jump.target()).name().as_ref().unwrap()[1..];
    format!("\tj {}\n", jump_target)
}