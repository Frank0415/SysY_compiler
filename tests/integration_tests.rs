use compiler::ast::{Block, Stmt, Expr};
use compiler::sysy;
use std::fs;

#[test]
fn test_case_1() {
    let input_path = "testcases/input/1.c";
    let input = fs::read_to_string(input_path).expect("Failed to read input file");

    let parser = sysy::CompUnitParser::new();
    let ast = parser.parse(&input).expect("Failed to parse input");

    // Basic verification that it's a valid AST
    assert_eq!(ast.func_def.ident, "main");
    
    let expected_block = Block {
        stmt: vec![Stmt::Return(Expr::Number(0))],
    };
    assert_eq!(ast.func_def.block, expected_block);
}
