use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    vec::Vec,
};

use strsim::normalized_damerau_levenshtein;

use crate::{arg_flag, flag, oof};

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
///
/// This function is also responsible for treating and checking the cli input
/// Like calling canonicale, checking if it exists.
pub fn parse_args() -> crate::Result<ParsedArgs> {
    let args = env::args_os().skip(1).collect();
    let mut parsed_args = parse_args_from(args)?;

    // If has a list of files, canonicalize them, reporting error if they do now exist
    match &mut parsed_args.command {
        Command::Compress { files, .. } | Command::Decompress { files, .. } => {
            *files = canonicalize_files(&files)?;
        },
        _ => {},
    }
    Ok(parsed_args)
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub command: Command,
    pub flags: oof::Flags,
}

/// check_for_typo checks if the first argument is
/// a typo for the compress subcommand.
/// Returns true if the arg is probably a typo or false otherwise.
fn is_typo<'a, P>(path: P) -> bool
where
    P: AsRef<Path> + 'a,
{
    if path.as_ref().exists() {
        // If the file exists then we won't check for a typo
        return false;
    }

    let path = path.as_ref().to_string_lossy();
    // We'll consider it a typo if the word is somewhat 'close' to "compress"
    normalized_damerau_levenshtein("compress", &path) > 0.625
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
                Err(io_err.into())
            }
        },
    }
}

fn canonicalize_files<'a, P>(files: &[P]) -> crate::Result<Vec<PathBuf>>
where
    P: AsRef<Path> + 'a,
{
    files.into_iter().map(canonicalize).collect()
}

pub fn parse_args_from(mut args: Vec<OsString>) -> crate::Result<ParsedArgs> {
    if oof::matches_any_arg(&args, &["--help", "-h"]) || args.is_empty() {
        return Ok(ParsedArgs { command: Command::ShowHelp, flags: oof::Flags::default() });
    }

    if oof::matches_any_arg(&args, &["--version"]) {
        return Ok(ParsedArgs { command: Command::ShowVersion, flags: oof::Flags::default() });
    }

    let subcommands = &["c", "compress"];
    let mut flags_info = vec![flag!('y', "yes"), flag!('n', "no")];

    let parsed_args = match oof::pop_subcommand(&mut args, subcommands) {
        Some(&"c") | Some(&"compress") => {
            // `ouch compress` subcommand
            let (args, flags) = oof::filter_flags(args, &flags_info)?;
            let mut files: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();

            if files.len() < 2 {
                return Err(crate::Error::MissingArgumentsForCompression);
            }

            // Safety: we checked that args.len() >= 2
            let compressed_output_path = files.pop().unwrap();

            let command = Command::Compress { files, compressed_output_path };
            ParsedArgs { command, flags }
        },
        // Defaults to decompression when there is no subcommand
        None => {
            flags_info.push(arg_flag!('o', "output"));

            if let Some(first_arg) = args.first() {
                if is_typo(first_arg) {
                    return Err(crate::Error::CompressionTypo);
                }
            } else {
                todo!("Complain that no decompression arguments were given.");
            }

            // Parse flags
            let (files, mut flags) = oof::filter_flags(args, &flags_info)?;
            let files = files.into_iter().map(PathBuf::from).collect();

            let output_folder = flags.take_arg("output").map(PathBuf::from);

            // TODO: ensure all files are decompressible

            let command = Command::Decompress { files, output_folder };
            ParsedArgs { command, flags }
        },
        _ => unreachable!("You should match each subcommand passed."),
    };

    Ok(parsed_args)
}
