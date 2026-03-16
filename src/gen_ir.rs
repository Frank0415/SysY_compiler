use crate::ast::UnaryOp;
use crate::ast::{Block, CompUnit, Exp, FuncDef, FuncFParam, RawType, Stmt};
use koopa::back::KoopaGenerator;
use koopa::ir::{builder_traits::*, types::*, *};
use std::fmt::Error;

/*
* Chap1: Process a single main function into a block
*/
pub fn gen_ir(cu: CompUnit) -> Result<Program, Error> {
    let mut program = Program::new();

    // Pattern match the CompUnit to extract the function definition
    let FuncDef {
        ident,
        func_params,
        block,
        ..
    } = cu.func_def;

    let func_name = format!("@{}", ident);

    let params: Vec<(Option<String>, Type)> = process_func_params(func_params);
    let ret_ty = type_to_ir(cu.func_def.func_type);
    let func = program.new_func(FunctionData::with_param_names(func_name, params, ret_ty));
    let func_data = program.func_mut(func);
    // let _arg1 = func_data.params()[0];
    // entry basic block
    let entry = func_data
        .dfg_mut()
        .new_bb()
        .basic_block(Some("%entry".into()));
    func_data
        .layout_mut()
        .bbs_mut()
        .push_key_back(entry)
        .unwrap();

    process_block(block, func_data, entry);

    Ok(program)
}

// enum Error {
//   InvalidArgs,
//   InvalidFile(io::Error),
//   Parse,
//   Io(io::Error),
// }

fn process_func_params(func_params: Vec<FuncFParam>) -> Vec<(Option<String>, Type)> {
    let mut params: Vec<(Option<String>, Type)> = Vec::new();
    for param in func_params {
        let typ = type_to_ir(param.bt);
        let name = format!("%{}", param.id);
        params.push((Some(name), typ));
    }
    params
}

pub trait ProcessIr {
    fn process_to_ir(&self, func_data: &mut FunctionData, bb: BasicBlock) -> Value;
}

impl ProcessIr for Exp {
    fn process_to_ir(&self, func_data: &mut FunctionData, bb: BasicBlock) -> Value {
        match self {
            Exp::Number(val) => func_data.dfg_mut().new_value().integer(*val),
            Exp::Unary { op, exp } => {
                let val = exp.process_to_ir(func_data, bb);
                match op {
                    UnaryOp::Plus => val,
                    UnaryOp::Minus => {
                        let zero = func_data.dfg_mut().new_value().integer(0);
                        let sub = func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::Sub, zero, val);
                        func_data
                            .layout_mut()
                            .bb_mut(bb)
                            .insts_mut()
                            .push_key_back(sub)
                            .unwrap();
                        sub
                    }
                    UnaryOp::Not => {
                        let zero = func_data.dfg_mut().new_value().integer(0);
                        let eq = func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::Eq, val, zero);
                        func_data
                            .layout_mut()
                            .bb_mut(bb)
                            .insts_mut()
                            .push_key_back(eq)
                            .unwrap();
                        eq
                    }
                }
            }
            Exp::Binary { op, lhs, rhs } => match op {
                crate::ast::BinaryOp::Land => {
                    let lhs_val = lhs.process_to_ir(func_data, bb);
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let lhs_bool =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::NotEq, lhs_val, zero);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(lhs_bool)
                        .unwrap();

                    let rhs_val = rhs.process_to_ir(func_data, bb);
                    let rhs_bool =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::NotEq, rhs_val, zero);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(rhs_bool)
                        .unwrap();

                    let res =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::And, lhs_bool, rhs_bool);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(res)
                        .unwrap();
                    res
                }
                crate::ast::BinaryOp::Lor => {
                    let lhs_val = lhs.process_to_ir(func_data, bb);
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let lhs_bool =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::NotEq, lhs_val, zero);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(lhs_bool)
                        .unwrap();

                    let rhs_val = rhs.process_to_ir(func_data, bb);
                    let rhs_bool =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::NotEq, rhs_val, zero);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(rhs_bool)
                        .unwrap();

                    let res =
                        func_data
                            .dfg_mut()
                            .new_value()
                            .binary(BinaryOp::Or, lhs_bool, rhs_bool);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(res)
                        .unwrap();
                    res
                }
                _ => {
                    let lhs_val = lhs.process_to_ir(func_data, bb);
                    let rhs_val = rhs.process_to_ir(func_data, bb);
                    let ir_op = match op {
                        crate::ast::BinaryOp::Add => BinaryOp::Add,
                        crate::ast::BinaryOp::Sub => BinaryOp::Sub,
                        crate::ast::BinaryOp::Mul => BinaryOp::Mul,
                        crate::ast::BinaryOp::Div => BinaryOp::Div,
                        crate::ast::BinaryOp::Mod => BinaryOp::Mod,
                        crate::ast::BinaryOp::Lt => BinaryOp::Lt,
                        crate::ast::BinaryOp::Gt => BinaryOp::Gt,
                        crate::ast::BinaryOp::Le => BinaryOp::Le,
                        crate::ast::BinaryOp::Ge => BinaryOp::Ge,
                        crate::ast::BinaryOp::Eq => BinaryOp::Eq,
                        crate::ast::BinaryOp::Neq => BinaryOp::NotEq,
                        _ => unreachable!(),
                    };
                    let bin = func_data
                        .dfg_mut()
                        .new_value()
                        .binary(ir_op, lhs_val, rhs_val);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(bin)
                        .unwrap();
                    bin
                }
            },
        }
    }
}

fn process_block(block: Block, func_data: &mut FunctionData, bb: BasicBlock) {
    for stmt in block.stmt {
        match stmt {
            Stmt::Return(exp) => {
                let ret_val = exp.process_to_ir(func_data, bb);
                let ret_inst = func_data.dfg_mut().new_value().ret(Some(ret_val));
                func_data
                    .layout_mut()
                    .bb_mut(bb)
                    .insts_mut()
                    .push_key_back(ret_inst)
                    .unwrap();
            }
        }
    }
}

fn type_to_ir(typ: RawType) -> Type {
    match typ {
        RawType::Int => Type::get_i32(),
        RawType::Null => Type::get_unit(),
    }
}

pub fn gen_text_ir(ir: &Program) -> String {
    let mut g = KoopaGenerator::new(Vec::new());
    g.generate_on(ir).unwrap();
    std::str::from_utf8(&g.writer()).unwrap().to_string()
}
