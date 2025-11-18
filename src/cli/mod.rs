//! CLI related functions, uses the clap argparsing definitions from `args.rs`.

mod args;

use std::{
    io,
    path::{Path, PathBuf},
};

use clap::Parser;
use fs_err as fs;

pub use self::args::{CliArgs, Subcommand};
use crate::{
    accessible::set_accessible,
    utils::{is_path_stdin, logger::set_log_display_level, threads::set_thread_count, FileVisibilityPolicy},
    QuestionPolicy,
};

impl CliArgs {
    /// A helper method that calls `clap::Parser::parse`.
    ///
    /// And:
    ///   1. Make paths absolute.
    ///   2. Checks the QuestionPolicy.
    pub fn parse_and_validate_args() -> crate::Result<(Self, QuestionPolicy, FileVisibilityPolicy)> {
        let mut args = Self::parse();

        set_accessible(args.accessible);
        set_log_display_level(args.quiet);

        match args.threads {
            Some(0) | None => {}
            Some(threads) => set_thread_count(threads),
        }

        let (Subcommand::Compress { files, .. }
        | Subcommand::Decompress { files, .. }
        | Subcommand::List { archives: files, .. }) = &mut args.cmd;
        *files = canonicalize_files(files)?;

        let skip_questions_positively = match (args.yes, args.no) {
            (false, false) => QuestionPolicy::Ask,
            (true, false) => QuestionPolicy::AlwaysYes,
            (false, true) => QuestionPolicy::AlwaysNo,
            (true, true) => unreachable!(),
        };

        let file_visibility_policy = FileVisibilityPolicy::new()
            .read_git_exclude(args.gitignore)
            .read_ignore(args.gitignore)
            .read_git_ignore(args.gitignore)
            .read_hidden(args.hidden);

        Ok((args, skip_questions_positively, file_visibility_policy))
    }
}

fn canonicalize_files(files: &[impl AsRef<Path>]) -> io::Result<Vec<PathBuf>> {
    files
        .iter()
        .map(|f| {
            if is_path_stdin(f.as_ref()) || f.as_ref().is_symlink() {
                Ok(f.as_ref().to_path_buf())
            } else {
                fs::canonicalize(f)
            }
        })
        .collect()
}
