use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    vec::Vec,
};

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

/// Calls parse_args_and_flags_from using std::env::args_os ( argv )
pub fn parse_args() -> crate::Result<ParsedArgs> {
    let args = env::args_os().skip(1).collect();
    parse_args_from(args)
}

pub struct ParsedArgs {
    pub command: Command,
    pub flags: oof::Flags,
}

fn canonicalize<'a, P>(path: P) -> crate::Result<PathBuf>
where
    P: AsRef<Path> + 'a,
{
    match std::fs::canonicalize(&path.as_ref()) {
        Ok(abs_path) => Ok(abs_path),
        Err(io_err) => {
            if !path.as_ref().exists() {
                Err(crate::Error::FileNotFound(PathBuf::from(path.as_ref())))
            } else {
                eprintln!("[ERROR] {}", io_err);
                Err(crate::Error::IoError)
            }
        }
    }
}

fn canonicalize_files<'a, P>(files: Vec<P>) -> crate::Result<Vec<PathBuf>>
where
    P: AsRef<Path> + 'a,
{
    files.into_iter().map(canonicalize).collect()
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
                return Err(crate::Error::MissingArgumentsForCompression);
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

            let files = args.into_iter().map(canonicalize);
            for file in files.clone() {
                if let Err(err) = file {
                    return Err(err);
                }
            }
            let files = files.map(Result::unwrap).collect();

            let output_folder = flags.take_arg("output").map(PathBuf::from);

            // TODO: ensure all files are decompressible

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
