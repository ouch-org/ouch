use std::path::PathBuf;

use clap::{Parser, ValueHint};

/// Command line options
#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Opts {
    /// Skip [Y/n] questions positively.
    #[clap(short, long, conflicts_with = "no")]
    pub yes: bool,

    /// Skip [Y/n] questions negatively.
    #[clap(short, long)]
    pub no: bool,

    /// Ouch and claps subcommands
    #[clap(subcommand)]
    pub cmd: Subcommand,
}

// CAREFUL: this docs can accidentally become part of the --help message if they get too long
// this was tested in clap 3.0.0-beta5.
/// Repository: https://github.com/ouch-org/ouch
//
// Ouch commands:
// - `compress`
// - `decompress`
// - `list`
//
// Clap commands:
//  - `help`
#[derive(Parser, PartialEq, Eq, Debug)]
pub enum Subcommand {
    /// Compress one or more files into one output file.
    #[clap(alias = "c")]
    Compress {
        /// Files to be compressed.
        #[clap(required = true, min_values = 1)]
        files: Vec<PathBuf>,

        /// The resulting file. Its extensions can be used to specify the compression formats.
        #[clap(required = true, value_hint = ValueHint::FilePath)]
        output: PathBuf,
    },
    /// Decompresses one or more files, optionally into another folder.
    #[clap(alias = "d")]
    Decompress {
        /// Files to be decompressed.
        #[clap(required = true, min_values = 1)]
        files: Vec<PathBuf>,

        /// Choose to  files in a directory other than the current
        #[clap(short = 'd', long = "dir", value_hint = ValueHint::DirPath)]
        output_dir: Option<PathBuf>,
    },
    /// List contents.     Alias: l
    #[clap(alias = "l")]
    List {
        /// Archives whose contents should be listed
        #[clap(required = true, min_values = 1)]
        archives: Vec<PathBuf>,

        /// Show archive contents as a tree
        #[clap(short, long)]
        tree: bool,
    },
}
