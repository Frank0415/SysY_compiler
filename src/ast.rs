#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub ident: String,
    pub func_params: Vec<FuncFParam>,
    pub block: Block,
}

#[derive(Debug)]
pub enum FuncType {
    Int,

}

#[derive(Debug)]
pub enum Type {
    Int,
}

#[derive(Debug)]
pub struct FuncFParam {
    pub bt: Type,
    pub id: String,
}


#[derive(Debug)]
pub enum Stmt {
    Return(Expr),  // 语句类型之一：返回语句
    // 后续扩展：Declare, Assign, If, While 等
}

#[derive(Debug)]
pub enum Expr {
    Number(i32)
}

#[derive(Debug)]
pub struct Block {
    pub stmt: Vec<Stmt>,
}