//! CLI arg parser configuration, command detection and input treatment.

use std::{
    path::{Path, PathBuf},
    vec::Vec,
};

use clap::{Parser, ValueHint};
use fs_err as fs;

pub use crate::utils::QuestionPolicy;
use crate::Error;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Opts {
    /// Skip overwrite questions positively.
    #[clap(short, long, conflicts_with = "no")]
    pub yes: bool,

    /// Skip overwrite questions negatively.
    #[clap(short, long)]
    pub no: bool,

    #[clap(subcommand)]
    pub cmd: Subcommand,
}

#[derive(Parser, PartialEq, Eq, Debug)]
pub enum Subcommand {
    /// Compress files.    Alias: c
    #[clap(alias = "c")]
    Compress {
        /// Files to be compressed
        #[clap(required = true, min_values = 1)]
        files: Vec<PathBuf>,

        /// The resulting file. Its extensions specify how the files will be compressed and they need to be supported
        #[clap(required = true, value_hint = ValueHint::FilePath)]
        output: PathBuf,
    },
    /// Compress files.    Alias: d
    #[clap(alias = "d")]
    Decompress {
        /// Files to be decompressed
        #[clap(required = true, min_values = 1)]
        files: Vec<PathBuf>,

        /// Decompress files in a directory other than the current
        #[clap(short, long = "dir", value_hint = ValueHint::DirPath)]
        output_dir: Option<PathBuf>,
    },
}

impl Opts {
    /// A helper method that calls `clap::Parser::parse` and then translates relative paths to absolute.
    /// Also determines if the user wants to skip questions or not
    pub fn parse_args() -> crate::Result<(Self, QuestionPolicy)> {
        let mut opts: Self = Self::parse();

        let (Subcommand::Compress { files, .. } | Subcommand::Decompress { files, .. }) = &mut opts.cmd;
        *files = canonicalize_files(files)?;

        let skip_questions_positively = if opts.yes {
            QuestionPolicy::AlwaysYes
        } else if opts.no {
            QuestionPolicy::AlwaysNo
        } else {
            QuestionPolicy::Ask
        };

        Ok((opts, skip_questions_positively))
    }
}

fn canonicalize(path: impl AsRef<Path>) -> crate::Result<PathBuf> {
    match fs::canonicalize(&path.as_ref()) {
        Ok(abs_path) => Ok(abs_path),
        Err(io_err) => {
            if !path.as_ref().exists() {
                Err(Error::FileNotFound(path.as_ref().into()))
            } else {
                Err(io_err.into())
            }
        }
    }
}

fn canonicalize_files(files: &[impl AsRef<Path>]) -> crate::Result<Vec<PathBuf>> {
    files.iter().map(canonicalize).collect()
}
