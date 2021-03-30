use std::{ffi::OsString, path::PathBuf};

const COMPRESS_SUBCOMMAND_FLAGS: [(&str, &str); 1] = [("-o", "--output")];
const DECOMPRESS_SUBCOMMAND_FLAGS: [(&str, &str); 0] = [];
const SUBCOMMNADS: [&str; 1] = ["compress"];

#[derive(Debug, PartialEq)]
pub enum Command {
    Compress {
        files: Vec<PathBuf>,
        output_folder: Option<PathBuf>,
    },
    Decompress {
        files: Vec<PathBuf>,
    },
    // Convert,
    ShowHelp,
    ShowVersion,
}

#[derive(Debug, PartialEq)]
pub enum ArgParsingError {
    InvalidFlags(Vec<OsString>),
}

pub mod argparsing {
    use std::{
        collections::HashMap,
        ffi::{OsStr, OsString},
        path::PathBuf,
    };

    // struct Opt {
    //     /// Activate debug mode
    //     // short and long flags (-d, --debug) will be deduced from the field's name
    //     // #[structopt(short, long)]
    //     debug: bool,

    //     /// Set speed
    //     // we don't want to name it "speed", need to look smart
    //     // #[structopt(short = "v", long = "velocity", default_value = "42")]
    //     speed: f64,

    //     /// Input file
    //     // #[structopt(parse(from_os_str))]
    //     input: PathBuf,

    //     /// Output file, stdout if not present
    //     // #[structopt(parse(from_os_str))]
    //     output: Option<PathBuf>,

    //     /// Where to write the output: to `stdout` or `file`
    //     // #[structopt(short)]
    //     out_type: String,

    //     /// File name: only required when `out-type` is set to `file`
    //     // #[structopt(name = "FILE", required_if("out-type", "file"))]
    //     file_name: Option<String>,
    // }

    struct NotVerifiedParsedArgs {
        args: Vec<OsString>,
        short_flags: HashMap<OsString, OsString>,
        long_flags: HashMap<OsString, OsString>,
    }

    pub enum IsFlagVariant {
        No,
        Short,
        Long,
    }

    pub fn is_flag(text: impl AsRef<OsStr>) -> IsFlagVariant {
        let text = text.as_ref();

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::ffi::OsStrExt;

            let bytes = text.as_bytes();

            debug_assert_eq!('-'.len_utf8(), 1);
            debug_assert_eq!(OsStr::new("--flag").as_bytes().get(0), Some(&b'-'));

            if let Some(b'-') = bytes.get(0) {
                if let Some(b'-') = bytes.get(1) {
                    IsFlagVariant::Long
                } else {
                    IsFlagVariant::Short
                }
            } else {
                IsFlagVariant::No
            }
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;

            let iter = text.encode_wide();

            // Note that hyphen_in_utf16 == 45_u16
            if let Some(45_u16) = iter.next() {
                if let Some(45_u16) = iter.next() {
                    IsFlagVariant::Long
                } else {
                    IsFlagVariant::Short
                }
            } else {
                IsFlagVariant::No
            }
        }
    }

    impl NotVerifiedParsedArgs {
        pub fn from_args(args: Vec<OsString>) -> Self {
            // let mut new_args = vec![];

            // let mut i = 0;
            // // If window[0] is flag, record into hashmap
            // while let Some(window) = args[i..].windows(2).next() {
            //     // If is a flag
            //     // TODO: change types with generic to simplify this if without needs of casting
            //     if let Some(index) = acceptable_positional_flags
            //         .iter()
            //         .position(|&(short, long)| *short == window[0] || *long == window[0])
            //     {
            //         let flag = &acceptable_positional_flags.get(index).expect("what?").1;
            //         // let flag = &acceptable_positional_flags[index].1;
            //         // If the flag was already passed
            //         if positional_flags.contains_key(OsStr::new(flag)) {
            //             panic!("duplicated flag, should this throw an error or just overwrite?");
            //         }

            //         // Register the flag
            //         positional_flags.insert(flag.into(), window[1].clone());
            //         i += 2;
            //     } else {
            //         // Isn't flag, so add to new_args
            //         new_args.push(window[0].clone());
            //         i += 1;
            //     }
            // }

            // if let Some(last) = args.last() {
            //     if acceptable_positional_flags
            //         .iter()
            //         .find(|&(short, long)| *short == last || *long == last)
            //         .is_some()
            //     {
            //         panic!("Flag requires positional argument, at last, no arguments were given...");
            //     }
            // }

            // if let Some(last_window) = args.rchunks(2).next() {
            //     if acceptable_positional_flags
            //         .iter()
            //         .find(|&(short, long)| *short == last_window[0] || *long == last_window[0])
            //         .is_none()
            //     {
            //         new_args.push(args.last().unwrap().clone());
            //     }
            // }

            // (new_args, positional_flags)
            todo!();
        }
    }

    // Says if any text matches any arg
    pub fn matches_any_arg(args: &[OsString], texts: &[&str]) -> bool {
        texts
            .iter()
            .any(|text| args.iter().find(|&arg| arg == text).is_some())
    }

    pub fn pop_subcommand(args: &mut Vec<OsString>, subcommands: &[&str]) -> Option<OsString> {
        if args.is_empty() {
            return None;
        }

        for subcommand in subcommands {
            if subcommand == &args[0] {
                let result = args.remove(0);
                return Some(result);
            }
        }
        None
    }

    pub fn filter_flags(
        args: Vec<OsString>,
        acceptable_positional_flags: &[(&str, &str)],
    ) -> (Vec<OsString>, HashMap<OsString, OsString>) {
        // let mut positional_flags = HashMap::<OsString, OsString>::new();

        // let mut new_args = vec![];

        // let mut i = 0;
        // // If window[0] is flag, record into hashmap
        // while let Some(window) = args[i..].windows(2).next() {
        //     // If is a flag
        //     // TODO: change types with generic to simplify this if without needs of casting
        //     if let Some(index) = acceptable_positional_flags
        //         .iter()
        //         .position(|&(short, long)| *short == window[0] || *long == window[0])
        //     {
        //         let flag = &acceptable_positional_flags.get(index).expect("what?").1;
        //         // let flag = &acceptable_positional_flags[index].1;
        //         // If the flag was already passed
        //         if positional_flags.contains_key(OsStr::new(flag)) {
        //             panic!("duplicated flag, should this throw an error or just overwrite?");
        //         }

        //         // Register the flag
        //         positional_flags.insert(flag.into(), window[1].clone());
        //         i += 2;
        //     } else {
        //         // Isn't flag, so add to new_args
        //         new_args.push(window[0].clone());
        //         i += 1;
        //     }
        // }

        // if let Some(last) = args.last() {
        //     if acceptable_positional_flags
        //         .iter()
        //         .find(|&(short, long)| *short == last || *long == last)
        //         .is_some()
        //     {
        //         panic!("Flag requires positional argument, at last, no arguments were given...");
        //     }
        // }

        // if let Some(last_window) = args.rchunks(2).next() {
        //     if acceptable_positional_flags
        //         .iter()
        //         .find(|&(short, long)| *short == last_window[0] || *long == last_window[0])
        //         .is_none()
        //     {
        //         new_args.push(args.last().unwrap().clone());
        //     }
        // }

        // (new_args, positional_flags)
        todo!();
    }
}

pub fn try_arg_parsing<I, T>(iter: I) -> Result<Command, ArgParsingError>
where
    T: Into<OsString>,
    I: IntoIterator<Item = T>,
{
    let mut args: Vec<OsString> = iter.into_iter().skip(1).map(Into::into).collect();

    if argparsing::matches_any_arg(&args, &["--help", "-h"]) || args.is_empty() {
        return Ok(Command::ShowHelp);
    }

    if argparsing::matches_any_arg(&args, &["--version", "-v"]) {
        return Ok(Command::ShowVersion);
    }

    match argparsing::pop_subcommand(&mut args, &SUBCOMMNADS) {
        Some(inner) if inner == "compress" => {
            let (args, positional_flags) =
                argparsing::filter_flags(args, &COMPRESS_SUBCOMMAND_FLAGS);

            let output_flag = OsString::from("--output");
            let output_flag = positional_flags
                .get(&output_flag)
                .clone()
                .map(PathBuf::from);

            return Ok(Command::Compress {
                files: args.into_iter().map(PathBuf::from).collect(),
                output_folder: output_flag,
            });
        }
        // Defaults to decompress subcommand
        None => {
            let (args, positional_flags) =
                argparsing::filter_flags(args, &DECOMPRESS_SUBCOMMAND_FLAGS);

            return Ok(Command::Decompress {
                files: args.into_iter().map(PathBuf::from).collect(),
            });
        }
        _ => unreachable!(),
    }

    // } else {
    //     if args.contains(&OsString::from("--output")) {
    //         // Shouldn't be in here, this flag is only for the compress command
    //         // return Command::UnexpectedFlag
    //         // TODO: this is bad, fix it
    //         return Err(ArgParsingError::InvalidFlags(vec!["--output".into()]));
    //     }

    //     let files = args.into_iter().map(PathBuf::from).collect();
    //     Ok(Command::Decompress { files })
    // }

    // let subcommands = ["decompress", "convert"];
    // let subcommand_detected = subcommands
    //     .iter()
    //     .find(|&subcommand| subcommand == first)
    //     .is_some();

    // // If there is no subcommand, defaults to subcommand "decompress"
    // if !subcommand_detected {
    //     args.insert(1, OsString::from("decompress"));
    // }
}

/*
Tests: should succeed (✅) and fail (❌):
```sh
✅ ouch                                # show help
✅ ouch --help                         # show help
✅ ouch anything --help in here        # show help
✅ ouch anything in here -h            # show help
✅ ouch --version                      # show version
✅ ouch --version anything             # show version
✅ ouch anything -v here -h            # show version
✅ ouch --version --help               # show help
❌ ouch --h                            # invalid flag
❌ ouch --v                            # invalid flag
❌ ouch -help                          # invalid flag (-e -l -p)
❌ ouch --any_other_flag               # invalid flag
✅ ouch compress a b c output.zip      # Ok subcommand compress
❌ ouch compress                       # Missing arguments
❌ ouch compress anything              # Missing last argument
✅ ouch --help compress anything       # show help
✅ ouch compress --help                # show help
✅ ouch anything -o anything           #   valid flag in this context
❌ ouch compress anything -o anything  # invalid flag in this context
❌ ouch anything -o                    # this flag requires   valid flag in this context
```

Pending:
 - Support `-y` and `-n`, as these options usage are not stabilized.
*/

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[allow(unused_imports)]
    use super::{try_arg_parsing, ArgParsingError, Command};
    // util for test the argument parsing
    macro_rules! test {
        ($expected_command:expr, $input_text:expr) => {{
            assert_eq!(
                $expected_command,
                try_arg_parsing($input_text.split_whitespace())
            )
        }};
    }

    #[test]
    fn test_arg_parsing_help() {
        let expected = Ok(Command::ShowHelp);
        test!(expected, "ouch");
        test!(expected, "ouch -h");
        test!(expected, "ouch --help");
        test!(expected, "ouch aaaaaaaa --help -o -e aaa");
        test!(expected, "ouch aaaaaaaa -h");
        test!(expected, "ouch --help compress aaaaaaaa");
        test!(expected, "ouch compress --help");
        test!(expected, "ouch --version --help");
        test!(expected, "ouch aaaaaaaa -v aaaa -h");
    }

    #[test]
    fn test_arg_parsing_version() {
        let expected = Ok(Command::ShowVersion);
        test!(expected, "ouch --version");
        test!(expected, "ouch a --version b");
        test!(expected, "ouch -v");
        test!(expected, "ouch aaaa aaa -v asdasd");
    }

    // #[test]
    // fn test_arg_parsing_invalid_flags() {
    //     let expected = Err(ArgParsingError::InvalidFlags(vec![]));
    //     test!(expected, "ouch -version");
    // }

    #[test]
    fn test_arg_parsing_compress_subcommand() {
        let files: Vec<PathBuf> = ["a", "b", "c"].iter().map(PathBuf::from).collect();

        let expected = Ok(Command::Compress {
            files: files.clone(),
            output_folder: None,
        });
        test!(expected, "ouch compress a b c");

        let expected = Ok(Command::Compress {
            files: files.clone(),
            output_folder: Some(PathBuf::from("folder")),
        });
        test!(expected, "ouch compress a b c --output folder");
        test!(expected, "ouch compress a b --output folder c");
        test!(expected, "ouch compress a --output folder b c");
        test!(expected, "ouch compress --output folder a b c");
    }

    #[test]
    fn test_arg_parsing_decompress_subcommand() {
        let files: Vec<PathBuf> = ["a", "b", "c"].iter().map(PathBuf::from).collect();

        let expected = Ok(Command::Decompress {
            files: files.clone(),
        });
        test!(expected, "ouch a b c");

        let files: Vec<PathBuf> = ["a", "b", "c", "d"].iter().map(PathBuf::from).collect();

        let expected = Ok(Command::Decompress {
            files: files.clone(),
        });
        test!(expected, "ouch a b c d");
    }
}
