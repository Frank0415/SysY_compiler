use std::{fmt::Error, future};

use clap::builder::Str;
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::{FunctionData, Program, Value};

use koopa::ir::ValueKind;

pub trait GenAsm {
    fn gen_asm(&self) -> Result<String, Error>;
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
        for (&_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);
                str += &translate_inst(value_data, self.dfg());
            }
        }
        Ok(str)
    }
}

fn translate_inst(value_data: &ValueData, dfg: &DataFlowGraph) -> String {
    match value_data.kind() {
        ValueKind::Integer(int) => {
            format!("\tli a0, {}\n", int.value())
        }
        ValueKind::Return(ret) => {
            let mut res = String::new();
            if let Some(v) = ret.value() {
                res += &translate_inst(dfg.value(v), dfg);
            }
            res += "\tret\n";
            res
        }
        _ => unreachable!(),
    }
}
