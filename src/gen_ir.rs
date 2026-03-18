use crate::ast::{Decl, UnaryOp};
use crate::ast::{Block, CompUnit, Exp, FuncDef, FuncFParam, RawType, Stmt, BlockItem, EvalExp};
use koopa::back::KoopaGenerator;
use koopa::ir::{builder_traits::*, types::*, *};
use std::fmt::Error;
use crate::gen_ir_exp::ProcessIr;
use crate::gen_ir_variables::{SymbolInfo, Variables};
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

    process_block(block, func_data, entry, &mut variable_maps);

    Ok(program)
}

fn process_func_params(func_params: Vec<FuncFParam>) -> Vec<(Option<String>, Type)> {
    let mut params: Vec<(Option<String>, Type)> = Vec::new();
    for param in func_params {
        let typ = type_to_ir(param.bt);
        let name = format!("%{}", param.id);
        params.push((Some(name), typ));
    }
    params
}


fn process_block(block: Block, func_data: &mut FunctionData, bb: BasicBlock, var_map: &mut Variables) {
    for stmt in block.stmt {
        match stmt {
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
                let typ = decl.typ;
                assert!(typ != RawType::Void, "Cannot declare void constant");
                for def in decl.defs {
                    let id = def.ident;
                    let val = def.init_val.exp.eval_exp(var_map);
                    var_map.insert(id,SymbolInfo::Const(val) );
                }
            }
        }
    }
}

fn type_to_ir(typ: RawType) -> Type {
    match typ {
        RawType::Int => Type::get_i32(),
        RawType::Void => Type::get_unit(),
    }
}