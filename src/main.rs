use compiler::gen_ir::{gen_ir, gen_text_ir};
use compiler::sysy;
use compiler::gen_asm::GenAsm;

use std::env::args;
use std::fs::{read_to_string, write};
use std::io::Result;
use std::process::exit;

fn main() -> Result<()> {
    // 解析命令行参数
    let mut args = args();
    args.next();
    let _mode = args.next().expect("Expected mode (e.g., -koopa)");
    let input = args.next().expect("Expected input file");
    args.next(); // Skip -o
    let output = args.next().expect("Expected output file");

    // 读取输入文件
    let input_content = read_to_string(&input)?;

    // 调用 lalrpop 生成的 parser 解析输入文件
    let parser = sysy::CompUnitParser::new();
    let ast = match parser.parse(&input_content) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("Error: failed to parse input file.");
            eprintln!("Details: {:?}", err);
            exit(1);
        }
    };

    // 输出解析得到的 AST
    println!("{:#?}", ast);
    let program = gen_ir(ast).unwrap();
    let text_form_ir = gen_text_ir(&program);
    println!("{}", text_form_ir);
    let asm = program.gen_asm().unwrap();
    write(output, asm)?;

    Ok(())
}
