use crate::ast::{Block, BlockItem, CompUnit, Exp, FuncDef, FuncFParam, RawType, Stmt};
use crate::gen_ir_variables::Variables;
use koopa::back::KoopaGenerator;
use koopa::ir::{builder_traits::*, types::*, *};
use std::fmt::Error;

pub trait ProcessIr {
    fn process_to_ir(
        &self,
        func_data: &mut FunctionData,
        bb: BasicBlock,
        var_map: &mut Variables,
    ) -> Value;
}

impl ProcessIr for Exp {
    fn process_to_ir(
        &self,
        func_data: &mut FunctionData,
        bb: BasicBlock,
        var_map: &mut Variables,
    ) -> Value {
        match self {
            Exp::Number(val) => func_data.dfg_mut().new_value().integer(*val),
            Exp::Unary { op, exp } => process_to_ir_unary(func_data, bb, op, exp, var_map),
            Exp::Binary { op, lhs, rhs } => {
                process_to_ir_binary(func_data, bb, op, lhs, rhs, var_map)
            }
            Exp::Var(variable) => process_to_ir_variable(func_data, bb, variable, var_map),
        }
    }
}

fn process_to_ir_variable(
    func_data: &mut FunctionData,
    _bb: BasicBlock,
    var: &String,
    var_map: &mut Variables,
) -> Value {
    // 1. 尝试作为常量获取
    if let Some(val) = var_map.get_const(var) {
        return func_data.dfg_mut().new_value().integer(val);
    }

    // 2. 如果不是常量，再作为变量获取 (当前的 get 方法只返回 Var 类型的 Value)
    var_map.get(var).expect(&format!("Undefined variable or constant: {}", var))
}
fn process_to_ir_unary(
    func_data: &mut FunctionData,
    bb: BasicBlock,
    op: &crate::ast::UnaryOp,
    exp: &Box<Exp>,
    var_map: &mut Variables,
) -> Value {
    let val = exp.process_to_ir(func_data, bb, var_map);
    match op {
        crate::ast::UnaryOp::Plus => val,
        crate::ast::UnaryOp::Minus => {
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
        crate::ast::UnaryOp::Not => {
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
fn process_to_ir_binary(
    func_data: &mut FunctionData,
    bb: BasicBlock,
    op: &crate::ast::BinaryOp,
    lhs: &Box<Exp>,
    rhs: &Box<Exp>,
    var_map: &mut Variables,
) -> Value {
    match op {
        crate::ast::BinaryOp::Land => {
            let lhs_val = lhs.process_to_ir(func_data, bb, var_map);
            let zero = func_data.dfg_mut().new_value().integer(0);
            let lhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, lhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(lhs_bool)
                .unwrap();

            let rhs_val = rhs.process_to_ir(func_data, bb, var_map);
            let rhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, rhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(rhs_bool)
                .unwrap();

            let res = func_data
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
            let lhs_val = lhs.process_to_ir(func_data, bb, var_map);
            let zero = func_data.dfg_mut().new_value().integer(0);
            let lhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, lhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(lhs_bool)
                .unwrap();

            let rhs_val = rhs.process_to_ir(func_data, bb, var_map);
            let rhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, rhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(rhs_bool)
                .unwrap();

            let res = func_data
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
            let lhs_val = lhs.process_to_ir(func_data, bb, var_map);
            let rhs_val = rhs.process_to_ir(func_data, bb, var_map);
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
    }
}
