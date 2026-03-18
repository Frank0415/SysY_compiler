pub mod arg;
pub mod ast;
pub mod debug;
pub mod gen_asm;
pub mod gen_ir;
mod reg_alloc;
mod gen_ir_variables;
mod gen_ir_exp;

use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub sysy);
