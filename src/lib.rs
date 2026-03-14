pub mod arg;
pub mod ast;
pub mod debug;
pub mod gen_asm;
pub mod gen_ir;
pub mod reg_alloc;
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub sysy);
