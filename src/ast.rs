/*
 * TODO 1. Global Variables
 * TODO 2. More Types
 */

/*
 * Some Explaination:
 * RawType instead of Type here:
 * is to prevent confusion from koopa::ir::Type
 */
use std::boxed::Box;

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

#[derive(PartialEq)]
pub enum Stmt {
    Return(Exp), // 语句类型之一：返回语句
                 // 后续扩展：Declare, Assign, If, While 等
}

#[derive(Debug, PartialEq)]
pub enum UnaryExp {}

#[derive(PartialEq)]
pub enum Exp {
    Number(i32),

    Unary {
        op: UnaryOp,
        exp: Box<Exp>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Exp>,
        rhs: Box<Exp>,
    },
}

#[derive(PartialEq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(PartialEq)]
pub struct Block {
    pub stmt: Vec<Stmt>,
}
