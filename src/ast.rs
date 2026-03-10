/*
 * TODO 1. Global Variables
 * TODO 2. More Types
 */

/*
 * Some Explaination:
 * RawType instead of Type here:
 * is to prevent confusion from koopa::ir::Type
 */

#[derive(Debug, PartialEq)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug, PartialEq)]
pub struct FuncDef {
    pub func_type: RawType,
    pub ident: String,
    pub func_params: Vec<FuncFParam>,
    pub block: Block,
}

#[derive(Debug, PartialEq)]
pub enum RawType {
    Int,
    Null,
}

#[derive(Debug, PartialEq)]
pub struct FuncFParam {
    pub bt: RawType,
    pub id: String,
}

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Return(Expr), // 语句类型之一：返回语句
                  // 后续扩展：Declare, Assign, If, While 等
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Number(i32),
}

#[derive(Debug, PartialEq)]
pub struct Block {
    pub stmt: Vec<Stmt>,
}
