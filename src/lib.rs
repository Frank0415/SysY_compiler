pub mod ast;
pub mod gen_ir;
pub mod gen_asm;
pub mod arg;
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub sysy);
