/*
 * TODO 1. Global Variables
 * TODO 2. More Types
 */

/*
 * Some Explaination:
 * RawType instead of Type here:
 * is to prevent confusion from koopa::ir::Type
 */
use crate::gen_ir_variables::Variables;
use std::boxed::Box;

#[derive(PartialEq)]
pub struct CompUnit {
    pub items: Vec<CompUnitItem>,
}

#[derive(PartialEq)]
pub enum CompUnitItem {
    Decl(Decl),
    FuncDef(FuncDef),
}

#[derive(PartialEq)]
pub struct FuncDef {
    pub func_type: RawType,
    pub ident: String,
    pub func_params: Vec<FuncFParam>,
    pub block: Block,
}

// #[derive(Debug, Clone)]
// pub enum GlobalDecl {
//     Const(ConstDecl),  // const int x = 1;
//     Var(VarDecl),      // int y = 2; 或 int z;（未初始化自动为0）
// }

#[derive(PartialEq)]
pub enum RawType {
    Int,
    Void,
}

#[derive(PartialEq)]
pub struct FuncFParam {
    pub bt: RawType,
    pub id: String,
}

#[derive(PartialEq)]
pub enum Stmt {
    Block(Block),
    Assign { lval: String, exp: Exp },
    Exp(Option<Exp>),    // 新增：[Exp] ";" 语句（若为 None 则是单独的空分号 ";"）
    Return(Option<Exp>), // 修改：将 Return(Exp) 改为返回 Option<Exp>，支持 "return;"
    IF(Box<IF>),
    WHILE(Box<WHILE>),
    Break,
    Continue,
}

#[derive(PartialEq)]
pub struct IF {
    pub cond: Exp,
    pub then_stmt: Stmt,
    pub else_stmt: Option<Stmt>,
}

#[derive(PartialEq)]
pub struct WHILE {
    pub cond: Exp,
    pub body_while: Stmt,
}
// #[derive(Debug, PartialEq)]
// pub enum UnaryExp {}

#[derive(PartialEq)]
pub enum Exp {
    Number(i32),
    Var(String), // 变量/常量引用
    Unary {
        op: UnaryOp,
        exp: Box<Exp>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Exp>,
        rhs: Box<Exp>,
    },
    Call {
        ident: String,
        args: Vec<Exp>,
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
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Neq,
    Land,
    Lor,
}

#[derive(PartialEq)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(PartialEq)]
pub struct Block {
    pub stmt: Vec<BlockItem>,
}

// Const Defs
#[derive(PartialEq)]
pub enum Decl {
    Const(ConstDecl),
    Var(VarDecl),
}

#[derive(PartialEq)]
pub struct ConstDecl {
    pub typ: RawType,
    pub defs: Vec<ConstDef>,
}
#[derive(PartialEq)]
pub struct ConstDef {
    pub ident: String,
    pub init_val: ConstExp,
}
#[derive(PartialEq)]
pub struct ConstExp {
    pub exp: Exp,
}
// Variable Exps
#[derive(PartialEq)]
pub struct VarDecl {
    pub typ: RawType,
    pub defs: Vec<VarDef>,
}
#[derive(PartialEq)]
pub struct VarDef {
    pub ident: String,
    pub init_val: Option<VarExp>,
}
// #[derive(PartialEq)]
// pub struct VarHasDef {
//     pub ident: String,
//     pub init_val: VarExp,
// }
#[derive(PartialEq)]
pub struct VarExp {
    pub exp: Exp,
}

pub trait EvalExp {
    fn eval_exp(&self, var_map: &Variables) -> i32;
}

impl EvalExp for Exp {
    fn eval_exp(&self, var_map: &Variables) -> i32 {
        match self {
            Exp::Number(n) => *n,
            Exp::Unary { op, exp } => {
                let val = exp.eval_exp(var_map);
                match op {
                    UnaryOp::Minus => -val,
                    UnaryOp::Not => {
                        if val == 0 {
                            1
                        } else {
                            0
                        }
                    }
                    _ => val,
                }
            }
            Exp::Var(name) => var_map.get_const(name).expect("Undefined constant"),
            Exp::Binary { op, lhs, rhs } => {
                let l = lhs.eval_exp(var_map);
                let r = rhs.eval_exp(var_map);
                match op {
                    BinaryOp::Add => l + r,
                    BinaryOp::Sub => l - r,
                    BinaryOp::Mul => l * r,
                    BinaryOp::Div => l / r,
                    BinaryOp::Mod => l % r,
                    BinaryOp::Lt => {
                        if l < r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Gt => {
                        if l > r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Le => {
                        if l <= r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Ge => {
                        if l >= r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Eq => {
                        if l == r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Neq => {
                        if l != r {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Land => {
                        if l != 0 && r != 0 {
                            1
                        } else {
                            0
                        }
                    }
                    BinaryOp::Lor => {
                        if l != 0 || r != 0 {
                            1
                        } else {
                            0
                        }
                    }
                }
            }
            Exp::Call { ident, args } => {
                panic!("Function call should not be evaluated at compile time!")
            }
        }
    }
}
