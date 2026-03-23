use crate::ast::{Block, BlockItem, CompUnit, EvalExp, Exp, FuncDef, FuncFParam, RawType, Stmt};
use crate::ast::{Decl, UnaryOp};
use crate::gen_ir_exp::ProcessIr;
use crate::gen_ir_variables::{SymbolInfo, Variables};
use koopa::back::KoopaGenerator;
use koopa::ir::builder::ValueInserter;
use koopa::ir::{builder_traits::*, types::*, *};
use std::fmt::Error;
/*
* Chap1: Process a single main function into a block
*/
pub fn gen_ir(cu: CompUnit) -> Result<Program, Error> {
    let mut variable_maps = Variables::new();
    let mut program = Program::new();

    process_func(&mut variable_maps, &mut program, cu);

    Ok(program)
}

fn process_func(var_map: &mut Variables, prog: &mut Program, cu: CompUnit) {
    var_map.enter_scope();
    // Pattern match the CompUnit to extract the function definition
    let FuncDef {
        ident,
        func_params,
        block,
        ..
    } = cu.func_def;

    let func_name = format!("@{}", ident);
    let params: Vec<(Option<String>, Type)> = process_func_params(func_params);
    let ret_ty = type_to_ir(&cu.func_def.func_type);
    let func = prog.new_func(FunctionData::with_param_names(func_name, params, ret_ty));
    let func_data = prog.func_mut(func);
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

    process_block(block, func_data, entry, var_map);
    var_map.exit_scope();
}

fn process_func_params(func_params: Vec<FuncFParam>) -> Vec<(Option<String>, Type)> {
    let mut params: Vec<(Option<String>, Type)> = Vec::new();
    for param in func_params {
        let typ = type_to_ir(&param.bt);
        let name = format!("%{}", param.id);
        params.push((Some(name), typ));
    }
    params
}

fn process_block(
    block: Block,
    func_data: &mut FunctionData,
    mut bb: BasicBlock,
    var_map: &mut Variables,
) -> BasicBlock {
    for item in block.stmt {
        bb = process_block_item(item, func_data, bb, var_map);
    }
    bb
}

fn process_block_item(
    item: BlockItem,
    func_data: &mut FunctionData,
    bb: BasicBlock,
    var_map: &mut Variables,
) -> BasicBlock {
    match item {
        BlockItem::Decl(Decl::Const(decl)) => {
            let _typ = type_to_ir(&decl.typ);
            for def in decl.defs {
                let id = def.ident;
                let val = def.init_val.exp.eval_exp(var_map);
                var_map.insert(id, SymbolInfo::Const(val));
            }
            bb
        }
        BlockItem::Decl(Decl::Var(decl)) => {
            let typ = type_to_ir(&decl.typ);
            for def in decl.defs {
                let id = def.ident;
                assert!(
                    !var_map.contains_in_current_scope(&id),
                    "Should not declare a variable multiple times in the same scope!"
                );
                let alloc_ptr = func_data.dfg_mut().new_value().alloc(typ.clone());
                func_data.dfg_mut().set_value_name(
                    alloc_ptr,
                    Some(format!("@{}_{}", id, var_map.get_scope_layer())),
                );
                func_data
                    .layout_mut()
                    .bb_mut(bb)
                    .insts_mut()
                    .push_key_back(alloc_ptr)
                    .unwrap();
                if let Some(init_val) = def.init_val {
                    let val = init_val.exp.process_to_ir(func_data, bb, var_map);
                    let store_inst = func_data.dfg_mut().new_value().store(val, alloc_ptr);
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(store_inst)
                        .unwrap();
                }
                var_map.insert(id, SymbolInfo::Var(alloc_ptr));
            }
            bb
        }
        BlockItem::Stmt(stmt) => process_stmt(stmt, func_data, bb, var_map),
    }
}

fn process_stmt(
    stmt: Stmt,
    func_data: &mut FunctionData,
    bb: BasicBlock,
    var_map: &mut Variables,
) -> BasicBlock {
    static mut BLOCK_COUNT: usize = 0;
    match stmt {
        Stmt::Assign { lval, exp } => {
            let val = exp.process_to_ir(func_data, bb, var_map);
            if let Some(dest) = var_map.get(&lval) {
                let store_inst = func_data.dfg_mut().new_value().store(val, dest);
                func_data
                    .layout_mut()
                    .bb_mut(bb)
                    .insts_mut()
                    .push_key_back(store_inst)
                    .unwrap();
            } else {
                panic!("Undefined variable: {}", lval);
            }
            bb
        }
        Stmt::Return(exp) => {
            let mut ret_val: Option<Value> = None;
            if let Some(expr) = exp {
                ret_val = Some(expr.process_to_ir(func_data, bb, var_map));
            }
            let ret_inst = func_data.dfg_mut().new_value().ret(ret_val);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(ret_inst)
                .unwrap();
            bb
        }
        Stmt::Exp(exp) => {
            if let Some(expr) = exp {
                let _ = expr.process_to_ir(func_data, bb, var_map);
            }
            bb
        }
        Stmt::Block(blk) => {
            var_map.enter_scope();
            let new_bb = process_block(blk, func_data, bb, var_map);
            var_map.exit_scope();
            new_bb
        }
        Stmt::IF(if_stmt) => {
            let count = unsafe {
                let c = BLOCK_COUNT;
                BLOCK_COUNT += 1;
                c
            };

            let then_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%then_{}", count).into()));
            let else_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%else_{}", count).into()));
            let end_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%end_{}", count).into()));

            let cond = if_stmt.cond.process_to_ir(func_data, bb, var_map);
            let br = func_data
                .dfg_mut()
                .new_value()
                .branch(cond, then_bb, else_bb);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(br)
                .unwrap();

            // Then branch
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(then_bb)
                .unwrap();
            let then_end_bb = process_stmt(if_stmt.then_stmt, func_data, then_bb, var_map);
            let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
            func_data
                .layout_mut()
                .bb_mut(then_end_bb)
                .insts_mut()
                .push_key_back(jump_end)
                .unwrap();

            // Else branch
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(else_bb)
                .unwrap();
            let else_end_bb = if let Some(else_stmt) = if_stmt.else_stmt {
                process_stmt(else_stmt, func_data, else_bb, var_map)
            } else {
                else_bb
            };
            let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
            func_data
                .layout_mut()
                .bb_mut(else_end_bb)
                .insts_mut()
                .push_key_back(jump_end)
                .unwrap();

            // End block
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(end_bb)
                .unwrap();
            end_bb
        }
    }
}

fn type_to_ir(typ: &RawType) -> Type {
    match typ {
        RawType::Int => Type::get_i32(),
        RawType::Void => Type::get_unit(),
    }
}
