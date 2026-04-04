use compiler::gen_asm::GenAsm;
use compiler::gen_ir::{gen_ir};
use compiler::sysy;

use std::env::args;
use std::fs::{read_to_string, write};
use std::io::Result;
use std::process::exit;
use koopa::back::KoopaGenerator;
use koopa::ir::{Program, Type};

fn main() -> Result<()> {
    Type::set_ptr_size(4);

    // 解析命令行参数
    let mut args = args();
    args.next();
    let mode = args.next().expect("Expected mode (e.g., -koopa)");
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

    let program = gen_ir(ast).unwrap();
    if mode == "-koopa" {
        let text_form_ir = gen_text_ir(&program);
        write(output, text_form_ir)?;
    } else if mode == "-riscv" {
        let asm = program.gen_asm().unwrap();
        write(output, asm)?;
    } else {
        eprintln!("Unsupported mode: {}", mode);
        exit(1);
    }

    Ok(())
}

pub fn gen_text_ir(ir: &Program) -> String {
    let mut g = KoopaGenerator::new(Vec::new());
    g.generate_on(ir).unwrap();
    std::str::from_utf8(&g.writer()).unwrap().to_string()
}

