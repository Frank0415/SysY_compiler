pub mod ast;
pub mod ir;
pub mod arg;
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(pub sysy);
