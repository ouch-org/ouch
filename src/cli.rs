//! CLI arg parser configuration, command detection and input treatment.

use std::{
    path::{Path, PathBuf},
    vec::Vec,
};

use clap::Parser;

use crate::{Error, Opts, QuestionPolicy, Subcommand};

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
    match std::fs::canonicalize(&path.as_ref()) {
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
