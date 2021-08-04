use ouch::{
    cli::{parse_args, ParsedArgs},
    commands, Result,
};

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        std::process::exit(ouch::EXIT_FAILURE);
    }
}

fn run() -> crate::Result<()> {
    let ParsedArgs { command, flags } = parse_args()?;
    commands::run(command, &flags)
}
