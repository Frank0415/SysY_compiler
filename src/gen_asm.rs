use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::values::Binary;
use koopa::ir::{FunctionData, Program, Value};
use std::fmt::Error;

use crate::reg_alloc::LinearScanAlloc;

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
        for (&_bb, node) in self.layout().bbs() {
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
                    match dfg.value(v).kind() {
                        ValueKind::Integer(int) => {
                            // 处理立即数返回值
                            res += &format!("\tli a0, {}\n", int.value());
                        }
                        ValueKind::Binary(_) => {
                            // 处理二元操作返回值
                            res += &format!("\tmv a0, {}\n", reg_alloc.get_reg(v).expect("Please implement stack feature")
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                res += "\tret\n";
                res
            }
            ValueKind::Binary(bin) => {
                let target: String;
                target = reg_alloc
                    .get_reg(*value)
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
                    _ => format!(
                        "placeholder for binary operation: {:?} op#1: {:?}, op#2: {:?}\n",
                        bin.op(),
                        bin.lhs(),
                        bin.rhs()
                    ),
                }
            }
            // ValueKind::Alloc(alloc) => {
            //     let ret: String;
            // }
            // ValueKind::Load(load) => {}
            // ValueKind::Store(store) => {}
            _ => unimplemented!(),
        }
    }
}

// 先尝试使用教程的汇编，而不是使用更简便的形式
fn process_eq_inst(
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

fn process_neq_inst(
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

fn process_le_inst(
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

fn process_ge_inst(
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

fn process_and_inst(
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

fn process_or_inst(
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

fn process_inst(
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

fn load_temp_int(
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
        leftreg = reg_alloc.get_reg(op1).cloned();
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
        rightreg = reg_alloc.get_reg(op2).cloned();
    }

    vec![Some(asm), leftreg, rightreg]
}
