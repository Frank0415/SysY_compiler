use std::collections::HashMap;
use crate::ast::Exp;
use crate::gen_ir_variables::Variables;
use koopa::ir::{builder_traits::*, *};

pub trait ProcessIr {
    fn process_to_ir(
        &self,
        func_data: &mut FunctionData,
        bb: &mut BasicBlock,
        var_map: &mut Variables,
        func_map: &HashMap<String, Function>,
    ) -> Value;
}

impl ProcessIr for Exp {
    fn process_to_ir(
        &self,
        func_data: &mut FunctionData,
        bb: &mut BasicBlock,
        var_map: &mut Variables,
        func_map: &HashMap<String, Function>,
    ) -> Value {
        match self {
            Exp::Number(val) => func_data.dfg_mut().new_value().integer(*val),
            Exp::Unary { op, exp } => process_to_ir_unary(func_data, bb, op, exp, var_map, func_map),
            Exp::Binary { op, lhs, rhs } => {
                process_to_ir_binary(func_data, bb, op, lhs, rhs, var_map, func_map)
            }
            Exp::Var(variable) => process_to_ir_variable(func_data, bb, variable, var_map),
            Exp::Call { ident, args } => process_to_ir_call(func_data, bb, var_map, func_map, ident, args),
        }
    }
}

fn process_to_ir_variable(
    func_data: &mut FunctionData,
    bb: &mut BasicBlock,
    var: &String,
    var_map: &mut Variables,
) -> Value {
    if let Some(val) = var_map.get_const(var) {
        return func_data.dfg_mut().new_value().integer(val);
    }

    if let Some(ptr) = var_map.get(var) {
        let load_inst = func_data.dfg_mut().new_value().load(ptr);
        func_data
            .layout_mut()
            .bb_mut(*bb)
            .insts_mut()
            .push_key_back(load_inst)
            .unwrap();
        return load_inst;
    }

    panic!("Undefined variable or constant: {}", var);
}

fn process_to_ir_call(
    func_data: &mut FunctionData,
    bb: &mut BasicBlock,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
    ident: &String,
    args: &Vec<Exp>,
) -> Value {
    for arg in args {}
    unimplemented!()
}

fn process_to_ir_unary(
    func_data: &mut FunctionData,
    bb: &mut BasicBlock,
    op: &crate::ast::UnaryOp,
    exp: &Box<Exp>,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
) -> Value {
    let val = exp.process_to_ir(func_data, bb, var_map, func_map);
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
                .bb_mut(*bb)
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
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(eq)
                .unwrap();
            eq
        }
    }
}

fn process_to_ir_binary(
    func_data: &mut FunctionData,
    bb: &mut BasicBlock,
    op: &crate::ast::BinaryOp,
    lhs: &Box<Exp>,
    rhs: &Box<Exp>,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
) -> Value {
    match op {
        crate::ast::BinaryOp::Land => {
            let id = var_map.get_id();
            let rhs_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%land_rhs_{}", id)));
            let end_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%land_end_{}", id)));

            let result_ptr = func_data.dfg_mut().new_value().alloc(Type::get_i32());
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(result_ptr)
                .unwrap();

            let lhs_val = lhs.process_to_ir(func_data, bb, var_map, func_map);
            let zero = func_data.dfg_mut().new_value().integer(0);
            let lhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, lhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(lhs_bool)
                .unwrap();

            let store_zero = func_data.dfg_mut().new_value().store(zero, result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(store_zero)
                .unwrap();

            let br = func_data
                .dfg_mut()
                .new_value()
                .branch(lhs_bool, rhs_bb, end_bb);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(br)
                .unwrap();

            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(rhs_bb)
                .unwrap();
            *bb = rhs_bb;

            let rhs_val = rhs.process_to_ir(func_data, bb, var_map, func_map);
            let rhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, rhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(rhs_bool)
                .unwrap();

            let store_rhs = func_data.dfg_mut().new_value().store(rhs_bool, result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(store_rhs)
                .unwrap();

            let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(jump_end)
                .unwrap();

            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(end_bb)
                .unwrap();
            *bb = end_bb;

            let load_res = func_data.dfg_mut().new_value().load(result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(load_res)
                .unwrap();
            load_res
        }
        crate::ast::BinaryOp::Lor => {
            let id = var_map.get_id();
            let rhs_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%lor_rhs_{}", id)));
            let end_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%lor_end_{}", id)));

            let result_ptr = func_data.dfg_mut().new_value().alloc(Type::get_i32());
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(result_ptr)
                .unwrap();

            let lhs_val = lhs.process_to_ir(func_data, bb, var_map, func_map);
            let zero = func_data.dfg_mut().new_value().integer(0);
            let one = func_data.dfg_mut().new_value().integer(1);
            let lhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, lhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(lhs_bool)
                .unwrap();

            let store_one = func_data.dfg_mut().new_value().store(one, result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(store_one)
                .unwrap();

            let br = func_data
                .dfg_mut()
                .new_value()
                .branch(lhs_bool, end_bb, rhs_bb);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(br)
                .unwrap();

            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(rhs_bb)
                .unwrap();
            *bb = rhs_bb;

            let rhs_val = rhs.process_to_ir(func_data, bb, var_map, func_map);
            let rhs_bool = func_data
                .dfg_mut()
                .new_value()
                .binary(BinaryOp::NotEq, rhs_val, zero);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(rhs_bool)
                .unwrap();

            let store_rhs = func_data.dfg_mut().new_value().store(rhs_bool, result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(store_rhs)
                .unwrap();

            let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(jump_end)
                .unwrap();

            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(end_bb)
                .unwrap();
            *bb = end_bb;

            let load_res = func_data.dfg_mut().new_value().load(result_ptr);
            func_data
                .layout_mut()
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(load_res)
                .unwrap();
            load_res
        }
        _ => {
            let lhs_val = lhs.process_to_ir(func_data, bb, var_map, func_map);
            let rhs_val = rhs.process_to_ir(func_data, bb, var_map, func_map);
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
                .bb_mut(*bb)
                .insts_mut()
                .push_key_back(bin)
                .unwrap();
            bin
        }
    }
}
