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

    variable_maps.enter_scope();
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

    process_block(block, func_data, entry, &mut variable_maps);

    Ok(program)
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
    bb: BasicBlock,
    var_map: &mut Variables,
) {
    for stmt in block.stmt {
        match stmt {
            BlockItem::Stmt(Stmt::Assign { lval, exp }) => {
                // 1. 计算右侧表达式的值
                let val = exp.process_to_ir(func_data, bb, var_map);

                // 2. 从变量表中获取该变量对应的 alloc 指令（指针）
                if let Some(dest) = var_map.get(&lval) {
                    // 3. 生成 store 指令：向指针写入值
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
            }
            BlockItem::Stmt(Stmt::Return(exp)) => {
                let ret_val = exp.process_to_ir(func_data, bb, var_map);
                let ret_inst = func_data.dfg_mut().new_value().ret(Some(ret_val));
                func_data
                    .layout_mut()
                    .bb_mut(bb)
                    .insts_mut()
                    .push_key_back(ret_inst)
                    .unwrap();
            }
            BlockItem::Decl(Decl::Const(decl)) => {
                let typ = type_to_ir(&decl.typ);
                assert_ne!(typ, Type::get_unit(), "Cannot declare void constant");
                for def in decl.defs {
                    let id = def.ident;
                    let val = def.init_val.exp.eval_exp(var_map);
                    var_map.insert(id, SymbolInfo::Const(val));
                }
            }
            BlockItem::Decl(Decl::Var(decl)) => {
                let typ = type_to_ir(&decl.typ);
                assert_ne!(typ, Type::get_unit(), "Cannot declare void variable");
                for def in decl.defs {
                    let id = def.ident;
                    assert_eq!(
                        var_map.get(&id),
                        None,
                        "Should not declare a variable multiple times!"
                    );
                    let alloc_ptr = func_data.dfg_mut().new_value().alloc(typ.clone());
                    func_data
                        .dfg_mut()
                        .set_value_name(alloc_ptr, Some(format!("@{}", id)));
                    func_data
                        .layout_mut()
                        .bb_mut(bb)
                        .insts_mut()
                        .push_key_back(alloc_ptr)
                        .unwrap();
                    // 2. 如果有初始值，生成 store 指令
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

                    // 3. 将变量名映射到这个指针 (alloc_ptr)
                    // 注意：SymbolInfo 需要能存储 Value 类型
                    var_map.insert(id, SymbolInfo::Var(alloc_ptr));
                }
            }
        }
    }
}
fn type_to_ir(typ: &RawType) -> Type {
    match typ {
        RawType::Int => Type::get_i32(),
        RawType::Void => Type::get_unit(),
    }
}
