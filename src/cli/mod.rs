//! CLI related functions, uses the clap argparsing definitions from `args.rs`.

mod args;

use std::path::{Path, PathBuf, absolute};

use clap::Parser;

pub use self::args::{CliArgs, Subcommand};
use crate::{
    QuestionPolicy, Result,
    accessible::set_accessible,
    utils::{
        FileVisibilityPolicy, canonicalize, is_path_stdin, logger::set_log_display_level, threads::set_thread_count,
    },
};

impl CliArgs {
    /// A helper method that calls `clap::Parser::parse`.
    ///
    /// And:
    ///   1. Make paths absolute.
    ///   2. Checks the QuestionPolicy.
    pub fn parse_and_validate_args() -> Result<(Self, QuestionPolicy, FileVisibilityPolicy)> {
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
        *files = absolutize_paths(files)?;

        let skip_questions_positively = match (args.yes, args.no) {
            (false, false) => QuestionPolicy::Ask,
            (true, false) => QuestionPolicy::AlwaysYes,
            (false, true) => QuestionPolicy::AlwaysNo,
            (true, true) => unreachable!(),
        };

        let follow_symlinks = matches!(
            &args.cmd,
            Subcommand::Compress {
                follow_symlinks: true,
                ..
            }
        );

        let file_visibility_policy = FileVisibilityPolicy::new()
            .read_git_exclude(args.gitignore)
            .read_ignore(args.gitignore)
            .read_git_ignore(args.gitignore)
            .read_hidden(args.hidden)
            .follow_symlinks(follow_symlinks);

        Ok((args, skip_questions_positively, file_visibility_policy))
    }
}

fn absolutize_paths(paths: &[impl AsRef<Path>]) -> Result<Vec<PathBuf>> {
    paths
        .iter()
        .map(|path| {
            let path = path.as_ref();
            if is_path_stdin(path) {
                Ok(path.into())
            } else if path.is_symlink() {
                Ok(absolute(path)?)
            } else {
                canonicalize(path)
            }
        })
        .collect()
}
