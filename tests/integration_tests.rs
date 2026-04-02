use compiler::ast::CompUnitItem;
use compiler::sysy;
use std::fs;

#[test]
fn test_case_1() {
    let input_path = "testcases/input/1.c";
    let input = fs::read_to_string(input_path).expect("Failed to read input file");

    let parser = sysy::CompUnitParser::new();
    let ast = parser.parse(&input).expect("Failed to parse input");

    // Basic verification that parser produced a top-level main function.
    let has_main = ast
        .items
        .iter()
        .any(|item| matches!(item, CompUnitItem::FuncDef(fd) if fd.ident == "main"));
    assert!(has_main, "Expected a top-level main function");

    // let expected_block = Block {
    //     stmt: vec![Stmt::Return(Exp::Number(0))],
    // };
    // assert_eq!(ast.func_def.block, expected_block);
}
