use clap::Parser;
use compiler::arg::Args;
use compiler::ir::ir_gen;
use compiler::sysy;
use koopa::back::KoopaGenerator;
use std::fs::read_to_string;
use std::io::Result;
use std::process::exit;

fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 读取输入文件
    let input_content = read_to_string(&args.input)?;

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
    let ir = ir_gen(ast).unwrap();
    let mut g = KoopaGenerator::new(Vec::new());
    g.generate_on(&ir).unwrap();
    let text_form_ir = std::str::from_utf8(&g.writer()).unwrap().to_string();
    println!("{}", text_form_ir);
    Ok(())
}
