use crate::ast::*;
use std::fmt::Debug;

impl Debug for Exp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exp::Number(n) => write!(f, "{}", n),
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
        }
    }
}

impl Debug for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::Return(exp) => write!(f, "return {:?};", exp),
        }
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        res += "Vec<Stmt>:[\n";
        let mut num = 0;
        for stmt in &self.stmt {
            res += &format!("  stmt#{}: {:?}\n", num, stmt);
            num += 1;
        }
        res += "]";
        write!(f, "{}", res)
    }
}
