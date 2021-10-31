use clap::{Parser, ValueHint};

use std::path::PathBuf;

/// Command line options
#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Opts {
    /// Skip overwrite questions positively.
    #[clap(short, long, conflicts_with = "no")]
    pub yes: bool,

    /// Skip overwrite questions negatively.
    #[clap(short, long)]
    pub no: bool,

    /// Action to take
    #[clap(subcommand)]
    pub cmd: Subcommand,
}

/// Actions to take
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
