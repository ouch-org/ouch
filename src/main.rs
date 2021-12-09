use ouch::{commands, Opts, Result};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(ouch::EXIT_FAILURE);
    }
}

fn run() -> Result<()> {
    let (args, skip_questions_positively) = Opts::parse_args()?;
    commands::run(args, skip_questions_positively)
}
