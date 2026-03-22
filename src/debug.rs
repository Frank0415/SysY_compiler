use crate::ast::*;
use std::fmt::Debug;

impl Debug for Exp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exp::Number(n) => write!(f, "{}", n),
            Exp::Var(s) => write!(f, "{}", s),
            Exp::Unary { op, exp } => write!(f, "({:?} {:?})", op, exp),
            Exp::Binary { op, lhs, rhs } => write!(f, "({:?} {:?} {:?})", lhs, op, rhs),
        }
    }
}

impl Debug for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Plus => write!(f, "+"),
            UnaryOp::Minus => write!(f, "-"),
            UnaryOp::Not => write!(f, "not"),
        }
    }
}

impl Debug for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Neq => write!(f, "!="),
            BinaryOp::Land => write!(f, "&&"),
            BinaryOp::Lor => write!(f, "||"),
        }
    }
}

impl Debug for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::Block(block) => write!(f, "{{ {:?} }}", block),
            Stmt::Assign { lval, exp } => write!(f, "{} = {:?};", lval, exp),
            Stmt::Exp(Some(exp)) => write!(f, "{:?};", exp),
            Stmt::Exp(None) => write!(f, ";"),
            Stmt::Return(Some(exp)) => write!(f, "return {:?};", exp),
            Stmt::Return(None) => write!(f, "return;"),
        }
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        res += "Vec<BlockItem>:[\n";
        for (num, item) in self.stmt.iter().enumerate() {
            res += &format!("  item#{}: {:?}\n", num, item);
        }
        res += "]";
        write!(f, "{}", res)
    }
}

impl Debug for BlockItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockItem::Decl(decl) => write!(f, "Decl: {:?}", decl),
            BlockItem::Stmt(stmt) => write!(f, "Stmt: {:?}", stmt),
        }
    }
}

impl Debug for Decl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decl::Const(decl) => write!(f, "{:?}", decl),
            Decl::Var(decl) => write!(f, "{:?}", decl),
        }
    }
}

impl Debug for ConstDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "const {:?} {:?}", self.typ, self.defs)
    }
}

impl Debug for ConstDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.ident, self.init_val)
    }
}

impl Debug for ConstExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.exp)
    }
}

impl Debug for VarDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Var {:?} {:?}", self.typ, self.defs)
    }
}

impl Debug for VarDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.init_val {
            None => write!(f, "{}, no evaluation!", self.ident),
            Some(varexp) => write!(f, "{} = {:?}", self.ident, varexp),
        }
    }
}

impl Debug for VarExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.exp)
    }
}

impl Debug for RawType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawType::Int => write!(f, "int"),
            RawType::Void => write!(f, "void"),
        }
    }
}

impl Debug for CompUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompUnit {{ {:?} }}", self.func_def)
    }
}

impl Debug for FuncDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncDef")
            .field("func_type", &self.func_type)
            .field("ident", &self.ident)
            .field("func_params", &self.func_params)
            .field("block", &self.block)
            .finish()
    }
}

impl Debug for FuncFParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {}", self.bt, self.id)
    }
}
