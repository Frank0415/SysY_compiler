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
    // You mentioned not to check if something is extra, so we just verify the core structure
}
