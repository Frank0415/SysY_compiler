use crate::ast::{Block, CompUnit, Expr, FuncDef, FuncFParam, RawType, Stmt};
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
    for param in func_params{
        let typ = type_to_ir(param.bt);
        let name = format!("%{}", param.id);
        params.push((Some(name), typ));
    }
    return params;
}

fn process_block(block: Block, func_data: &mut FunctionData, bb: BasicBlock) {
    for stmt in block.stmt {
        match stmt {
            Stmt::Return(Expr::Number(val)) => {
                // 提取出的数字 0
                let ret_val: Value = func_data.dfg_mut().new_value().integer(val);

                // 创建 Return 指令
                let ret_inst = func_data.dfg_mut().new_value().ret(Some(ret_val));

                // 将指令插入基本块
                func_data
                    .layout_mut()
                    .bb_mut(bb)
                    .insts_mut()
                    .push_key_back(ret_inst)
                    .unwrap();
            } // 后续扩展可在 match 中添加更多语句提取逻辑
        }
    }
}

fn type_to_ir(typ: RawType) -> types::Type {
    match typ {
        RawType::Int => Type::get_i32(),
        default => Type::get_unit(),
    }
}

pub fn gen_text_ir(ir: &Program) -> String {
    let mut g = KoopaGenerator::new(Vec::new());
    g.generate_on(ir).unwrap();
    std::str::from_utf8(&g.writer()).unwrap().to_string()
}
