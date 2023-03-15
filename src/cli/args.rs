use std::{ffi::OsString, path::PathBuf};

use clap::{Parser, ValueHint};

// Ouch command line options (docstrings below are part of --help)
/// A command-line utility for easily compressing and decompressing files and directories.
///
/// Supported formats: tar, zip, gz, xz/lzma, bz/bz2, lz4, sz, zst.
///
/// Repository: https://github.com/ouch-org/ouch
#[derive(Parser, Debug)]
#[command(about, version)]
// Disable rustdoc::bare_urls because rustdoc parses URLs differently than Clap
#[allow(rustdoc::bare_urls)]
pub struct CliArgs {
    /// Skip [Y/n] questions positively
    #[arg(short, long, conflicts_with = "no", global = true)]
    pub yes: bool,

    /// Skip [Y/n] questions negatively
    #[arg(short, long, global = true)]
    pub no: bool,

    /// Activate accessibility mode, reducing visual noise
    #[arg(short = 'A', long, env = "ACCESSIBLE", global = true)]
    pub accessible: bool,

    /// Ignores hidden files
    #[arg(short = 'H', long, global = true)]
    pub hidden: bool,

    /// Silences output
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Ignores files matched by git's ignore files
    #[arg(short = 'g', long, global = true)]
    pub gitignore: bool,

    /// Specify the format of the archive
    #[arg(short, long, global = true)]
    pub format: Option<OsString>,

    /// Ouch and claps subcommands
    #[command(subcommand)]
    pub cmd: Subcommand,

    /// Compression raw level as each algo has
    #[arg(short = 'l', long)]
    pub level: Option<i16>,
}

#[derive(Parser, PartialEq, Eq, Debug)]
#[allow(rustdoc::bare_urls)]
pub enum Subcommand {
    /// Compress one or more files into one output file
    #[command(visible_alias = "c")]
    Compress {
        /// Files to be compressed
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,

        /// The resulting file. Its extensions can be used to specify the compression formats
        #[arg(required = true, value_hint = ValueHint::FilePath)]
        output: PathBuf,
    },
    /// Decompresses one or more files, optionally into another folder
    #[command(visible_alias = "d")]
    Decompress {
        /// Files to be decompressed
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,

        /// Place results in a directory other than the current one
        #[arg(short = 'd', long = "dir", value_hint = ValueHint::DirPath)]
        output_dir: Option<PathBuf>,
    },
    /// List contents of an archive
    #[command(visible_aliases = ["l", "ls"])]
    List {
        /// Archives whose contents should be listed
        #[arg(required = true, num_args = 1..)]
        archives: Vec<PathBuf>,

        /// Show archive contents as a tree
        #[arg(short, long)]
        tree: bool,
    },
}
