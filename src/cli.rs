//! CLI related functions, uses the clap argparsing definitions from `opts.rs`.

use std::{
    io,
    path::{Path, PathBuf},
    vec::Vec,
};

use clap::Parser;
use fs_err as fs;

use crate::{accessible::set_accessible, utils::FileVisibilityPolicy, Opts, QuestionPolicy, Subcommand};

impl Opts {
    /// A helper method that calls `clap::Parser::parse`.
    ///
    /// And:
    ///   1. Make paths absolute.
    ///   2. Checks the QuestionPolicy.
    pub fn parse_args() -> crate::Result<(Self, QuestionPolicy, FileVisibilityPolicy)> {
        let mut opts = Self::parse();

        set_accessible(opts.accessible);

        let (Subcommand::Compress { files, .. }
        | Subcommand::Decompress { files, .. }
        | Subcommand::List { archives: files, .. }) = &mut opts.cmd;
        *files = canonicalize_files(files)?;

        let skip_questions_positively = match (opts.yes, opts.no) {
            (false, false) => QuestionPolicy::Ask,
            (true, false) => QuestionPolicy::AlwaysYes,
            (false, true) => QuestionPolicy::AlwaysNo,
            (true, true) => unreachable!(),
        };

        // TODO: change this to be just a single function call?
        let file_visibility_policy = FileVisibilityPolicy::new()
            .read_git_exclude(opts.gitignore)
            .read_ignore(opts.gitignore)
            .read_git_ignore(opts.gitignore)
            .read_hidden(opts.hidden);

        Ok((opts, skip_questions_positively, file_visibility_policy))
    }
}

fn canonicalize_files(files: &[impl AsRef<Path>]) -> io::Result<Vec<PathBuf>> {
    files.iter().map(fs::canonicalize).collect()
}
