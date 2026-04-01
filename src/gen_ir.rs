use crate::ast::Decl;
use crate::ast::{
    Block, BlockItem, CompUnit, CompUnitItem, EvalExp, FuncDef, RawType, Stmt,
};
use crate::gen_ir_exp::ProcessIr;
use crate::gen_ir_variables::{SymbolInfo, Variables};
use koopa::ir::{builder_traits::*, types::*, *};
use std::collections::HashMap;
use std::fmt::Error;
/*
* Chap1: Process a single main function into a block
*/
pub fn gen_ir(cu: CompUnit) -> Result<Program, Error> {
    let mut variable_maps = Variables::new();
    variable_maps.enter_scope();
    let mut program = Program::new();
    let mut function_maps: HashMap<String, Function> = HashMap::new();

    add_sysy_lib_decls(&mut variable_maps, &mut program, &mut function_maps);

    // pass1: 先把所有函数原型建出来，拿到 Function 句柄
    for item in &cu.items {
        if let CompUnitItem::FuncDef(fd) = item {
            let params: Vec<(Option<String>, Type)> = fd
                .func_params
                .iter()
                .map(|p| (Some(format!("%{}", p.id)), type_to_ir(&p.bt)))
                .collect();

            let func_name = format!("@{}", fd.ident);
            let ret_ty = type_to_ir(&fd.func_type);
            let f = program.new_func(FunctionData::with_param_names(func_name, params, ret_ty));

            assert!(
                function_maps.insert(fd.ident.clone(), f).is_none(),
                "duplicate function definition: {}",
                fd.ident
            );
            assert!(
                !variable_maps.contains_in_current_scope(&fd.ident),
                "duplicate global symbol: {}",
                fd.ident
            );
            variable_maps.insert(fd.ident.clone(), SymbolInfo::Func);
        }
    }

    for item in cu.items {
        match item {
            CompUnitItem::FuncDef(func_def) => {
                process_func(&mut variable_maps, &mut program, func_def, &function_maps);
            }
            CompUnitItem::Decl(_decl) => {
                unimplemented!();
                // TODO: Implement global declarations
                // For now, parsing is supported but IR generation is a placeholder
            }
        }
    }

    Ok(program)
}

fn add_sysy_lib_decls(
    var_map: &mut Variables,
    program: &mut Program,
    function_maps: &mut HashMap<String, Function>,
) {
    let i32_ty = Type::get_i32();
    let ptr_i32_ty = Type::get_pointer(Type::get_i32());
    let unit_ty = Type::get_unit();

    let lib_funcs: Vec<(&str, Vec<Type>, Type)> = vec![
        ("getint", vec![], i32_ty.clone()),
        ("getch", vec![], i32_ty.clone()),
        ("getarray", vec![ptr_i32_ty.clone()], i32_ty),
        ("putint", vec![Type::get_i32()], unit_ty.clone()),
        ("putch", vec![Type::get_i32()], unit_ty.clone()),
        (
            "putarray",
            vec![Type::get_i32(), ptr_i32_ty],
            unit_ty.clone(),
        ),
        ("starttime", vec![], unit_ty.clone()),
        ("stoptime", vec![], unit_ty),
    ];

    for (name, params, ret_ty) in lib_funcs {
        assert!(
            !var_map.contains_in_current_scope(name),
            "duplicate global symbol: {}",
            name
        );
        let param_defs: Vec<(Option<String>, Type)> =
            params.into_iter().map(|t| (None, t)).collect();
        let func = program.new_func(FunctionData::with_param_names(
            format!("@{}", name),
            param_defs,
            ret_ty,
        ));
        assert!(
            function_maps.insert(name.to_string(), func).is_none(),
            "duplicate function definition: {}",
            name
        );
        var_map.insert(name.to_string(), SymbolInfo::Func);
    }
}

fn process_func(var_map: &mut Variables, prog: &mut Program, func_def: FuncDef, func_map: &HashMap<String, Function>) {
    var_map.enter_scope();
    // Pattern match the FuncDef to extract details
    let FuncDef {
        ident,
        func_params,
        block,
                                                   func_type,
    } = func_def;

    let param_defs: Vec<(String, RawType)> =
        func_params.into_iter().map(|p| (p.id, p.bt)).collect();

    let func = *func_map
        .get(&ident)
        .unwrap_or_else(|| panic!("Undefined function: {}", ident));
    let ret_ty = type_to_ir(&func_type);
    let is_void_return = ret_ty.is_unit();
    let func_data = prog.func_mut(func);

    // Variable maps
    let incoming: Vec<Value> = func_data.params().to_vec();

    let entry = func_data
        .dfg_mut()
        .new_bb()
        .basic_block(Some("%entry".into()));
    func_data
        .layout_mut()
        .bbs_mut()
        .push_key_back(entry)
        .unwrap();

    for (i, (id, bt)) in param_defs.iter().enumerate() {
        assert!(
            !var_map.contains_in_current_scope(id) && !var_map.contains_in_global_scope(id),
            "duplicate parameter/global symbol name in function scope"
        );
        let ptr = func_data.dfg_mut().new_value().alloc(type_to_ir(bt));
        func_data
            .layout_mut()
            .bb_mut(entry)
            .insts_mut()
            .push_key_back(ptr)
            .unwrap();

        let st = func_data.dfg_mut().new_value().store(incoming[i], ptr);
        func_data
            .layout_mut()
            .bb_mut(entry)
            .insts_mut()
            .push_key_back(st)
            .unwrap();

        var_map.insert(id.clone(), SymbolInfo::Var(ptr));
    }

    let end_bb = process_block(block, func_data, entry, var_map, func_map);
    if !is_bb_terminated(func_data, &end_bb) {
        if is_void_return {
            let ret_inst = func_data.dfg_mut().new_value().ret(None);
            func_data
                .layout_mut()
                .bb_mut(end_bb)
                .insts_mut()
                .push_key_back(ret_inst)
                .unwrap();
        } else {
            panic!("Non-void function should end with a return statement!");
        }
    }
    var_map.exit_scope();
}

fn process_block(
    block: Block,
    func_data: &mut FunctionData,
    mut bb: BasicBlock,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
) -> BasicBlock {
    for item in block.stmt {
        bb = process_block_item(item, func_data, bb, var_map, func_map);
        if is_bb_terminated(func_data, &bb) {
            break;
        }
    }
    bb
}

fn process_block_item(
    item: BlockItem,
    func_data: &mut FunctionData,
    bb: BasicBlock,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
) -> BasicBlock {
    match item {
        BlockItem::Decl(Decl::Const(decl)) => {
            let _typ = type_to_ir(&decl.typ);
            for def in decl.defs {
                let id = def.ident;
                assert!(
                    !var_map.contains_in_current_scope(&id) && !var_map.contains_in_global_scope(&id),
                    "Should not declare a constant multiple times in the same scope or conflict with globals!"
                );
                let val = def.init_val.exp.eval_exp(var_map);
                var_map.insert(id, SymbolInfo::Const(val));
            }
            bb
        }
        BlockItem::Decl(Decl::Var(decl)) => {
            let typ = type_to_ir(&decl.typ);
            let mut current_bb = bb;
            for def in decl.defs {
                let id = def.ident;
                assert!(
                    !var_map.contains_in_current_scope(&id) && !var_map.contains_in_global_scope(&id),
                    "Should not declare a variable multiple times in the same scope or conflict with globals!"
                );
                let alloc_ptr = func_data.dfg_mut().new_value().alloc(typ.clone());
                let unique_id = var_map.get_id();
                func_data
                    .dfg_mut()
                    .set_value_name(alloc_ptr, Some(format!("@{}_{}", id, unique_id)));
                func_data
                    .layout_mut()
                    .bb_mut(current_bb)
                    .insts_mut()
                    .push_key_back(alloc_ptr)
                    .unwrap();
                if let Some(init_val) = def.init_val {
                    let val = init_val
                        .exp
                        .process_to_ir(func_data, &mut current_bb, var_map, func_map);
                    let store_inst = func_data.dfg_mut().new_value().store(val, alloc_ptr);
                    func_data
                        .layout_mut()
                        .bb_mut(current_bb)
                        .insts_mut()
                        .push_key_back(store_inst)
                        .unwrap();
                }
                var_map.insert(id, SymbolInfo::Var(alloc_ptr));
            }
            current_bb
        }
        BlockItem::Stmt(stmt) => process_stmt(stmt, func_data, bb, var_map, func_map),
    }
}

fn process_stmt(
    stmt: Stmt,
    func_data: &mut FunctionData,
    bb: BasicBlock,
    var_map: &mut Variables,
    func_map: &HashMap<String, Function>,
) -> BasicBlock {
    match stmt {
        Stmt::Assign { lval, exp } => {
            let mut current_bb = bb;
            let val = exp.process_to_ir(func_data, &mut current_bb, var_map, func_map);
            if let Some(dest) = var_map.get(&lval) {
                let store_inst = func_data.dfg_mut().new_value().store(val, dest);
                func_data
                    .layout_mut()
                    .bb_mut(current_bb)
                    .insts_mut()
                    .push_key_back(store_inst)
                    .unwrap();
            } else {
                panic!("Undefined variable: {}", lval);
            }
            current_bb
        }
        Stmt::Return(exp) => {
            let mut current_bb = bb;
            let mut ret_val: Option<Value> = None;
            if let Some(expr) = exp {
                ret_val = Some(expr.process_to_ir(func_data, &mut current_bb, var_map, func_map));
            }
            let ret_inst = func_data.dfg_mut().new_value().ret(ret_val);
            func_data
                .layout_mut()
                .bb_mut(current_bb)
                .insts_mut()
                .push_key_back(ret_inst)
                .unwrap();
            current_bb
        }
        Stmt::Exp(exp) => {
            let mut current_bb = bb;
            if let Some(expr) = exp {
                let _ = expr.process_to_ir(func_data, &mut current_bb, var_map, func_map);
            }
            current_bb
        }
        Stmt::Block(blk) => {
            var_map.enter_scope();
            let new_bb = process_block(blk, func_data, bb, var_map, func_map);
            var_map.exit_scope();
            new_bb
        }
        Stmt::IF(if_stmt) => {
            let id = var_map.get_id();

            let then_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%then_{}", id)));
            let else_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%else_{}", id)));
            let end_bb = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%end_{}", id)));

            let mut current_bb = bb;
            let cond = if_stmt
                .cond
                .process_to_ir(func_data, &mut current_bb, var_map, func_map);
            let br = func_data
                .dfg_mut()
                .new_value()
                .branch(cond, then_bb, else_bb);
            func_data
                .layout_mut()
                .bb_mut(current_bb)
                .insts_mut()
                .push_key_back(br)
                .unwrap();

            // Then branch
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(then_bb)
                .unwrap();
            let then_end_bb = process_stmt(if_stmt.then_stmt, func_data, then_bb, var_map, func_map);
            if !is_bb_terminated(func_data, &then_end_bb) {
                let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
                func_data
                    .layout_mut()
                    .bb_mut(then_end_bb)
                    .insts_mut()
                    .push_key_back(jump_end)
                    .unwrap();
            }

            // Else branch
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(else_bb)
                .unwrap();
            let else_end_bb = if let Some(else_stmt) = if_stmt.else_stmt {
                process_stmt(else_stmt, func_data, else_bb, var_map, func_map)
            } else {
                else_bb
            };
            if !is_bb_terminated(func_data, &else_end_bb) {
                let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
                func_data
                    .layout_mut()
                    .bb_mut(else_end_bb)
                    .insts_mut()
                    .push_key_back(jump_end)
                    .unwrap();
            }

            // End block
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(end_bb)
                .unwrap();
            end_bb
        }
        Stmt::WHILE(while_stmt) => {
            let id = var_map.get_id();

            let while_entry = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%while_ent_{}", id)));
            let while_body = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%while_body_{}", id)));
            let while_end = func_data
                .dfg_mut()
                .new_bb()
                .basic_block(Some(format!("%while_end_{}", id)));

            // current block jumps to entry
            let jump_into_entry = func_data.dfg_mut().new_value().jump(while_entry);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(jump_into_entry)
                .unwrap();

            // while_entry block
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(while_entry)
                .unwrap();

            // Push jump context for break/continue
            var_map.enter_while(&while_entry, &while_end);

            let mut curr_entry_bb = while_entry;
            let cond = while_stmt
                .cond
                .process_to_ir(func_data, &mut curr_entry_bb, var_map, func_map);
            let br = func_data
                .dfg_mut()
                .new_value()
                .branch(cond, while_body, while_end);
            func_data
                .layout_mut()
                .bb_mut(curr_entry_bb)
                .insts_mut()
                .push_key_back(br)
                .unwrap();

            // while_body block
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(while_body)
                .unwrap();
            let body_end_bb = process_stmt(while_stmt.body_while, func_data, while_body, var_map, func_map);
            if !is_bb_terminated(func_data, &body_end_bb) {
                let jump_back_stmt = func_data.dfg_mut().new_value().jump(while_entry);
                func_data
                    .layout_mut()
                    .bb_mut(body_end_bb)
                    .insts_mut()
                    .push_key_back(jump_back_stmt)
                    .unwrap();
            }

            // Pop jump context
            var_map.exit_while();

            // while_end block
            func_data
                .layout_mut()
                .bbs_mut()
                .push_key_back(while_end)
                .unwrap();

            while_end
        }
        Stmt::Break => {
            let break_block = var_map
                .get_break()
                .expect("break statement outside of while loop");
            let jump = func_data.dfg_mut().new_value().jump(break_block);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(jump)
                .unwrap();
            bb
        }
        Stmt::Continue => {
            let cont_block = var_map
                .get_continue()
                .expect("continue statement outside of while loop");
            let jump = func_data.dfg_mut().new_value().jump(cont_block);
            func_data
                .layout_mut()
                .bb_mut(bb)
                .insts_mut()
                .push_key_back(jump)
                .unwrap();
            bb
        }
    }
}

fn type_to_ir(typ: &RawType) -> Type {
    match typ {
        RawType::Int => Type::get_i32(),
        RawType::Void => Type::get_unit(),
    }
}

fn is_bb_terminated(func_data: &mut FunctionData, bb: &BasicBlock) -> bool {
    for this_bb_data in func_data.layout().bbs().iter() {
        if this_bb_data.0 == bb {
            return if let Some(last_inst) = this_bb_data.1.insts().back_key() {
                matches!(
                    func_data.dfg().value(*last_inst).kind(),
                    ValueKind::Return(_) | ValueKind::Jump(_) | ValueKind::Branch(_)
                )
            } else {
                false
            };
        }
    }
    false
}
