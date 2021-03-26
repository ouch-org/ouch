use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

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
    UnknownFlags(Vec<OsString>),
}

pub struct Flag {
    short: OsString,
    long: OsString,
}

impl Flag {
    pub fn new(short: impl Into<OsString>, long: impl Into<OsString>) -> Self {
        Flag {
            short: short.into(),
            long: long.into(),
        }
    }
}

pub fn filter_flags(
    args: Vec<OsString>,
    _boolean_flags: Vec<Flag>,
    positional_flags: Vec<Flag>,
) -> (Vec<OsString>, HashMap<OsString, OsString>) {
    //
    todo!()
}

impl Command {
    pub fn from<I, T>(iter: I) -> Self
    where
        T: Into<OsString>,
        I: IntoIterator<Item = T>,
    {
        let args: Vec<OsString> = iter.into_iter().skip(1).map(Into::into).collect();

        // Says if any text matches any arg
        let matches_any_arg = |texts: &[&str]| -> bool {
            texts
                .iter()
                .any(|text| args.iter().find(|&arg| arg == text).is_some())
        };

        if matches_any_arg(&["--version", "-v"]) {
            return Self::ShowVersion;
        }

        if matches_any_arg(&["--help", "-h"]) || args.is_empty() {
            return Self::ShowHelp;
        }

        let boolean_flags = vec![];
        let positional_flags = vec![Flag::new("-o", "--output")];

        let (args, flags) = filter_flags(args, boolean_flags, positional_flags);

        // Safe: we checked for args.is_empty()
        let first = &args[0];

        if first == "compress" {
            let output_folder = flags.get(OsStr::new("--output")).map(PathBuf::from);
            let files = args.into_iter().map(PathBuf::from).collect();

            Command::Compress {
                files,
                output_folder,
            }
        } else {
            if args.contains(&OsString::from("--output")) {
                // Shouldn't be in here, this flag is only for the compress command
                // return Command::UnexpectedFlag
                // TODO: this is bad, fix it
                return Command::UnknownFlags(vec!["--output".into()]);
            }

            let files = args.into_iter().map(PathBuf::from).collect();
            Command::Decompress { files }
        }

        // let subcommands = ["decompress", "convert"];
        // let subcommand_detected = subcommands
        //     .iter()
        //     .find(|&subcommand| subcommand == first)
        //     .is_some();

        // // If there is no subcommand, defaults to subcommand "decompress"
        // if !subcommand_detected {
        //     args.insert(1, OsString::from("decompress"));
        // }

        // // We guaranteed that
        // let subcommand = args[0].to_str().unwrap();

        // if condition {
        //     unimplemented!();
        // }

        // if matches() {
        // unimplemented!();
        // }

        // We
        //

        // if matches(&args, &["--help", "-h"]) {
        //     return Self::ShowHelp;
        // }
        // println!("{:?}", matches(&args, &["compress", "decompress"]));

        // //
    }
}
