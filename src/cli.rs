use std::{env, ffi::OsString, io, path::PathBuf, vec::Vec};

use oof::{arg_flag, flag};

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    /// Files to be compressed
    Compress {
        files: Vec<PathBuf>,
        compressed_output_path: PathBuf,
    },
    /// Files to be decompressed and their extensions
    Decompress {
        files: Vec<PathBuf>,
        output_folder: Option<PathBuf>,
    },
    ShowHelp,
    ShowVersion,
}

#[derive(PartialEq, Eq, Debug)]
pub struct CommandInfo {
    pub command: Command,
    pub flags: oof::Flags,
    // pub config: Config, // From .TOML, maybe, in the future
}

/// Calls parse_args_and_flags_from using std::env::args_os ( argv )
pub fn parse_args() -> crate::Result<ParsedArgs> {
    let args = env::args_os().skip(1).collect();
    parse_args_from(args)
}

pub struct ParsedArgs {
    pub command: Command,
    pub flags: oof::Flags,
    // pub program_called: OsString, // Useful?
}

fn canonicalize_files(files: Vec<PathBuf>) -> io::Result<Vec<PathBuf>> {
    files.into_iter().map(|path| path.canonicalize()).collect()
}

pub fn parse_args_from(mut args: Vec<OsString>) -> crate::Result<ParsedArgs> {
    if oof::matches_any_arg(&args, &["--help", "-h"]) || args.is_empty() {
        return Ok(ParsedArgs {
            command: Command::ShowHelp,
            flags: oof::Flags::default(),
        });
    }

    if oof::matches_any_arg(&args, &["--version"]) {
        return Ok(ParsedArgs {
            command: Command::ShowVersion,
            flags: oof::Flags::default(),
        });
    }

    let subcommands = &["compress"];

    let mut flags_info = vec![flag!('y', "yes"), flag!('n', "no")];

    let parsed_args = match oof::pop_subcommand(&mut args, subcommands) {
        Some(&"compress") => {
            let (args, flags) = oof::filter_flags(args, &flags_info)?;
            let mut files: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();

            if files.len() < 2 {
                panic!("The compress subcommands demands at least 2 arguments, see usage:.......");
            }
            // Safety: we checked that args.len() >= 2
            let compressed_output_path = files.pop().unwrap();

            let files = canonicalize_files(files)?;

            let command = Command::Compress {
                files,
                compressed_output_path,
            };
            ParsedArgs { command, flags }
        }
        // Defaults to decompression when there is no subcommand
        None => {
            flags_info.push(arg_flag!('o', "output"));

            // Parse flags
            let (args, mut flags) = oof::filter_flags(args, &flags_info)?;

            let files: Vec<_> = args.into_iter().map(PathBuf::from).collect();
            let output_folder = flags.take_arg("output").map(PathBuf::from);

            // Is the output here fully correct?
            // With the paths not canonicalized?

            let command = Command::Decompress {
                files,
                output_folder,
            };
            ParsedArgs { command, flags }
        }
        _ => unreachable!("You should match each subcommand passed."),
    };

    Ok(parsed_args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_args(text: &str) -> Vec<OsString> {
        let args = text.split_whitespace();
        args.map(OsString::from).collect()
    }

    // // util for test the argument parsing
    // macro_rules! test {
    //     ($expected_command:expr, $input_text:expr) => {{
    //         assert_eq!(
    //             $expected_command,
    //             oof::try_arg_parsing($input_text.split_whitespace())
    //         )
    //     }};
    // }

    macro_rules! parse {
        ($input_text:expr) => {{
            let args = gen_args($input_text);
            parse_args_from(args).unwrap()
        }};
    }

    #[test]
    // The absolute flags that ignore all the other argparsing rules are --help and --version
    fn test_absolute_flags() {
        let expected = Command::ShowHelp;
        assert_eq!(expected, parse!("").command);
        assert_eq!(expected, parse!("-h").command);
        assert_eq!(expected, parse!("--help").command);
        assert_eq!(expected, parse!("aaaaaaaa --help -o -e aaa").command);
        assert_eq!(expected, parse!("aaaaaaaa -h").command);
        assert_eq!(expected, parse!("--help compress aaaaaaaa").command);
        assert_eq!(expected, parse!("compress --help").command);
        assert_eq!(expected, parse!("--version --help").command);
        assert_eq!(expected, parse!("aaaaaaaa -v aaaa -h").command);

        let expected = Command::ShowVersion;
        assert_eq!(expected, parse!("ouch --version").command);
        assert_eq!(expected, parse!("ouch a --version b").command);
    }

    #[test]
    fn test_arg_parsing_compress_subcommand() {
        let files = ["a", "b", "c"].iter().map(PathBuf::from).collect();

        let expected = Command::Compress {
            files,
            compressed_output_path: "d".into(),
        };
        assert_eq!(expected, parse!("compress a b c d").command);
    }

    #[test]
    fn test_arg_parsing_decompress_subcommand() {
        let files: Vec<_> = ["a", "b", "c"].iter().map(PathBuf::from).collect();

        let expected = Command::Decompress {
            files: files.clone(),
            output_folder: None,
        };
        assert_eq!(expected, parse!("a b c").command);

        let expected = Command::Decompress {
            files,
            output_folder: Some("folder".into()),
        };
        assert_eq!(expected, parse!("a b c --output folder").command);
        assert_eq!(expected, parse!("a b --output folder c").command);
        assert_eq!(expected, parse!("a --output folder b c").command);
        assert_eq!(expected, parse!("--output folder a b c").command);
    }

    // #[test]
    // fn test_arg_parsing_decompress_subcommand() {
    //     let files: Vec<PathBuf> = ["a", "b", "c"].iter().map(PathBuf::from).collect();

    //     let expected = Ok(Command::Decompress {
    //         files: files.clone(),
    //     });
    //     test!(expected, "ouch a b c");

    //     let files: Vec<PathBuf> = ["a", "b", "c", "d"].iter().map(PathBuf::from).collect();

    //     let expected = Ok(Command::Decompress {
    //         files: files.clone(),
    //     });
    //     test!(expected, "ouch a b c d");
    // }
}
