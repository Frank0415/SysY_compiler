use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Mode -koopa followed by the input file
    /// We use an alias to allow -koopa (single dash) specifically
    #[arg(
        short = 'k',
        long = "koopa",
        value_name = "INPUT",
        help = "Emit Koopa IR and specify input path"
    )]
    pub input: String,

    /// Output file path
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    pub output: String,
}
