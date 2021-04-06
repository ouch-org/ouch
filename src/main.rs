use ouch::{
    cli::{parse_args, ParsedArgs},
    evaluator::Evaluator,
    Result,
};

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        std::process::exit(127);
    }
}

fn run() -> crate::Result<()> {
    let ParsedArgs { command, flags } = parse_args()?;
    Evaluator::evaluate(command, &flags)
}
