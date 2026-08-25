use clap::Command;

const COMPRESSION_POSITIONAL_HELP: &str = "Input files; or output archive with compression format when placed last";

pub fn with_combined_compress_positionals(mut command: Command) -> Command {
    let compress = command
        .find_subcommand_mut("compress")
        .expect("compress subcommand should exist");

    *compress = std::mem::take(compress).mut_args(|argument| match argument.get_id().as_str() {
        "files" | "output" => argument.help(COMPRESSION_POSITIONAL_HELP),
        _ => argument,
    });

    command
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;
    use crate::cli::CliArgs;

    #[test]
    fn keeps_standard_compress_positional_descriptions() {
        let command = CliArgs::command();
        let compress = command
            .find_subcommand("compress")
            .expect("compress subcommand should exist");
        let descriptions = compress
            .get_arguments()
            .filter(|argument| matches!(argument.get_id().as_str(), "files" | "output"))
            .map(|argument| argument.get_help().map(ToString::to_string))
            .collect::<Vec<_>>();

        assert_eq!(
            descriptions,
            [
                Some("Files to be compressed".to_owned()),
                Some("The resulting file. Its extensions can be used to specify the compression formats".to_owned()),
            ]
        );
    }

    #[test]
    fn combines_compress_positional_descriptions() {
        let command = with_combined_compress_positionals(CliArgs::command());
        let compress = command
            .find_subcommand("compress")
            .expect("compress subcommand should exist");

        for id in ["files", "output"] {
            let argument = compress
                .get_arguments()
                .find(|argument| argument.get_id() == id)
                .expect("compress positional argument should exist");
            assert_eq!(
                argument.get_help().map(ToString::to_string).as_deref(),
                Some(COMPRESSION_POSITIONAL_HELP)
            );
        }
    }
}
