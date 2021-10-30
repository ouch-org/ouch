use ouch::{cli::Opts, commands, Result};

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        std::process::exit(ouch::EXIT_FAILURE);
    }
}

fn run() -> Result<()> {
    let (args, skip_questions_positively) = Opts::parse_args()?;
    commands::run(args, skip_questions_positively)
}
