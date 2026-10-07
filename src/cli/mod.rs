//! CLI related functions, uses the clap argparsing definitions from `args.rs`.

mod args;
#[cfg(test)]
mod completion;

use std::path::{Path, PathBuf, absolute};

use clap::Parser;

pub use self::args::{CliArgs, Subcommand};
use crate::{
    FinalError, QuestionPolicy, Result,
    accessible::set_accessible,
    error::Error,
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

        // Clap only catches flag clashes given at the same level, flags split
        // around the subcommand (`ouch --yes d --no`) slip through, so refuse
        // those combinations here instead of panicking.
        if args.rename && (args.yes || args.no) {
            return Err(Error::Custom {
                reason: FinalError::with_title("Cannot combine --rename with --yes or --no")
                    .detail("--rename already answers every conflict by renaming")
                    .hint("Drop --yes/--no and keep only --rename"),
            });
        }
        if args.yes && args.no {
            return Err(Error::Custom {
                reason: FinalError::with_title("Cannot combine --yes with --no")
                    .detail("The two flags answer every question in opposite ways")
                    .hint("Pass only one of --yes or --no"),
            });
        }

        let skip_questions_positively = match (args.yes, args.no, args.rename) {
            (false, false, false) => QuestionPolicy::Ask,
            (true, false, false) => QuestionPolicy::AlwaysYes,
            (false, true, false) => QuestionPolicy::AlwaysNo,
            (false, false, true) => QuestionPolicy::AlwaysRename,
            _ => unreachable!("flag clashes rejected above"),
        };

        let (hidden, gitignore, follow_symlinks) = match &args.cmd {
            Subcommand::Compress {
                hidden,
                gitignore,
                follow_symlinks,
                ..
            } => (*hidden, *gitignore, *follow_symlinks),
            Subcommand::Decompress { .. } | Subcommand::List { .. } => (false, false, false),
        };

        let file_visibility_policy = FileVisibilityPolicy::new()
            .read_git_exclude(gitignore)
            .read_ignore(gitignore)
            .read_git_ignore(gitignore)
            .read_hidden(hidden)
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
