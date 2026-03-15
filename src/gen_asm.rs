use std::fmt::Error;

use koopa::ir::ValueKind;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::values::Binary;
use koopa::ir::{FunctionData, Program, Value};

use crate::reg_alloc::{self, LinearScanAlloc};

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
                        ValueKind::Integer(int) => { // 处理立即数返回值
                            res += &format!("\tli a0, {}\n", int.value());
                        }
                        ValueKind::Binary(_) => { // 处理二元操作返回值
                            res += &format!("\tmv a0, {}\n", reg_alloc.get_reg(v).expect("Please implement stack feature")); // 假设结果在 t5 寄存器中
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
                let op = match bin.op() {
                    koopa::ir::BinaryOp::Add => return process_add_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Sub => return process_sub_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Mul => return process_mul_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Div => return process_div_inst(bin, dfg, reg_alloc, target),
                    koopa::ir::BinaryOp::Eq => return process_eq_inst(bin, dfg, reg_alloc, target),
                    _ => unimplemented!(),
                };
                format!(
                    "placeholder for binary operation: {} op#1: {:?}, op#2: {:?}\n",
                    op,
                    bin.lhs(),
                    bin.rhs()
                )
            }
            _ => unreachable!(),
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

fn process_sub_inst(
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
    asm += &format!("\tsub\t{}, {}, {}\n", target, leftreg, rightreg);
    asm
}

fn process_add_inst(
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
    asm += &format!("\tadd\t{}, {}, {}\n", target, leftreg, rightreg);
    asm
}

fn process_mul_inst(
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
    asm += &format!("\tmul\t{}, {}, {}\n", target, leftreg, rightreg);
    asm
}

fn process_div_inst(
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
    asm += &format!("\tdiv\t{}, {}, {}\n", target, leftreg, rightreg);
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
