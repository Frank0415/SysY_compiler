use crate::reg_alloc::LinearScanAlloc;
use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::values::{Alloc, Binary, Load, Store};

// 先尝试使用教程的汇编，而不是使用更简便的形式
pub fn process_eq_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    asm += &format!("\txor\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    asm
}

pub fn process_neq_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    asm += &format!("\txor\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tsnez\t{}, {}\n", target, target);
    asm
}

pub fn process_le_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    // a <= b is !(a > b) -> !(sgt target, left, right)
    asm += &format!("\tsgt\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    asm
}

pub fn process_ge_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    // a >= b is !(a < b) -> !(slt target, left, right)
    asm += &format!("\tslt\t{}, {}, {}\n", target, leftreg, rightreg);
    asm += &format!("\tseqz\t{}, {}\n", target, target);
    asm
}

pub fn process_and_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    // Logical AND: (lhs != 0) && (rhs != 0)
    asm += &format!("\tsnez\tt0, {}\n", leftreg);
    asm += &format!("\tsnez\tt1, {}\n", rightreg);
    asm += &format!("\tand\t{}, t0, t1\n", target);
    asm
}

pub fn process_or_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();
    // Logical OR: (lhs != 0) || (rhs != 0)
    asm += &format!("\tsnez\tt0, {}\n", leftreg);
    asm += &format!("\tsnez\tt1, {}\n", rightreg);
    asm += &format!("\tor\t{}, t0, t1\n", target);
    asm
}

pub fn process_inst(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
    target: String,
    inst: &str,
) -> String {
    let mut asm: String = String::new();
    let s = load_temp_int(bin, dfg, reg_alloc);
    asm += &s[0].as_ref().unwrap();
    let leftreg = s[1].as_ref().unwrap();
    let rightreg = s[2].as_ref().unwrap();

    asm += &format!("\t{}\t{}, {}, {}\n", inst, target, leftreg, rightreg);
    asm
}

pub fn load_temp_int(
    bin: &Binary,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> Vec<Option<String>> {
    let op1 = bin.lhs();
    let op2 = bin.rhs();

    let mut asm: String = String::new();
    let leftreg: Option<String>;
    let rightreg: Option<String>;

    // Handle OP1
    if let ValueKind::Integer(int) = dfg.value(op1).kind() {
        let val = int.value();
        if val != 0 {
            leftreg = Some(String::from("t5"));
            asm += &format!("\tli\tt5, {}\n", val);
        } else {
            leftreg = Some(String::from("x0"));
        }
    } else {
        // Look up register from allocator
        leftreg = reg_alloc.get_reg(&op1).cloned();
    }

    // Handle OP2
    if let ValueKind::Integer(int) = dfg.value(op2).kind() {
        let val = int.value();
        if val != 0 {
            rightreg = Some(String::from("t6"));
            asm += &format!("\tli\tt6, {}\n", val);
        } else {
            rightreg = Some(String::from("x0"));
        }
    } else {
        // Look up register from allocator
        rightreg = reg_alloc.get_reg(&op2).cloned();
    }

    vec![Some(asm), leftreg, rightreg]
}

pub fn process_alloc_inst() -> String {
    String::new()
}

pub fn process_load_inst(
    load: &Load,
    inst: &koopa::ir::Value,
    dfg: &DataFlowGraph,
    reg_alloc: &LinearScanAlloc,
) -> String {
    let mut ret = String::new();
    let src = load.src();
    let src_offset = reg_alloc
        .get_stack(&src)
        .expect("Source not found on stack") - 4;
    let dest_reg = reg_alloc.get_reg(inst);
    let dest_stack = reg_alloc.get_stack(inst);

    if let Some(reg) = dest_reg {
        ret += &format!("\tlw {}, {}(sp)\n", reg, src_offset);
    } else if let Some(stack) = dest_stack {
        ret += &format!("\tlw t0, {}(sp)\n", src_offset);
        ret += &format!("\tsw t0, {}(sp)\n", stack);
    } else {
        panic!("Load destination has neither register nor stack slot");
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
    let dest_offset = reg_alloc.get_stack(&dest).expect("Dest not found on stack") - 4;

    if let koopa::ir::ValueKind::Integer(int) = dfg.value(val).kind() {
        let v = int.value();
        if v == 0 {
            ret += &format!("\tsw x0, {}(sp)\n", dest_offset);
        } else {
            ret += &format!("\tli t0, {}\n", v);
            ret += &format!("\tsw t0, {}(sp)\n", dest_offset);
        }
    } else if let Some(reg) = reg_alloc.get_reg(&val) {
        ret += &format!("\tsw {}, {}(sp)\n", reg, dest_offset);
    } else if let Some(stack) = reg_alloc.get_stack(&val) {
        ret += &format!("\tlw t0, {}(sp)\n", stack);
        ret += &format!("\tsw t0, {}(sp)\n", dest_offset);
    } else {
        panic!("Store value has neither register nor stack slot");
    }

    ret
}
