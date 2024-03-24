use std::{ffi::OsString, path::PathBuf};

use clap::{Parser, ValueHint};

// Ouch command line options (docstrings below are part of --help)
/// A command-line utility for easily compressing and decompressing files and directories.
///
/// Supported formats: tar, zip, gz, 7z, xz/lzma, bz/bz2, lz4, sz (Snappy), zst and rar.
///
/// Repository: https://github.com/ouch-org/ouch
#[derive(Parser, Debug, PartialEq)]
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

    /// decompress or list with password
    #[arg(short = 'p', long = "password", global = true)]
    pub password: Option<String>,

    // Ouch and claps subcommands
    #[command(subcommand)]
    pub cmd: Subcommand,
}

#[derive(Parser, PartialEq, Eq, Debug)]
#[allow(rustdoc::bare_urls)]
pub enum Subcommand {
    /// Compress one or more files into one output file
    #[command(visible_alias = "c")]
    Compress {
        /// Files to be compressed
        #[arg(required = true, value_hint = ValueHint::FilePath)]
        files: Vec<PathBuf>,

        /// The resulting file. Its extensions can be used to specify the compression formats
        #[arg(required = true, value_hint = ValueHint::FilePath)]
        output: PathBuf,

        /// Compression level, applied to all formats
        #[arg(short, long, group = "compression-level")]
        level: Option<i16>,

        /// Fastest compression level possible,
        /// conflicts with --level and --slow
        #[arg(long, group = "compression-level")]
        fast: bool,

        /// Slowest (and best) compression level possible,
        /// conflicts with --level and --fast
        #[arg(long, group = "compression-level")]
        slow: bool,
    },
    /// Decompresses one or more files, optionally into another folder
    #[command(visible_alias = "d")]
    Decompress {
        /// Files to be decompressed
        #[arg(required = true, num_args = 1.., value_hint = ValueHint::FilePath)]
        files: Vec<PathBuf>,

        /// Place results in a directory other than the current one
        #[arg(short = 'd', long = "dir", value_hint = ValueHint::FilePath)]
        output_dir: Option<PathBuf>,
    },
    /// List contents of an archive
    #[command(visible_aliases = ["l", "ls"])]
    List {
        /// Archives whose contents should be listed
        #[arg(required = true, num_args = 1.., value_hint = ValueHint::FilePath)]
        archives: Vec<PathBuf>,

        /// Show archive contents as a tree
        #[arg(short, long)]
        tree: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_splitter(input: &str) -> impl Iterator<Item = &str> {
        input.split_whitespace()
    }

    fn to_paths(iter: impl IntoIterator<Item = &'static str>) -> Vec<PathBuf> {
        iter.into_iter().map(PathBuf::from).collect()
    }

    macro_rules! test {
        ($args:expr, $expected:expr) => {
            let result = match CliArgs::try_parse_from(args_splitter($args)) {
                Ok(result) => result,
                Err(err) => panic!(
                    "CLI result is Err, expected Ok, input: '{}'.\nResult: '{err}'",
                    $args
                ),
            };
            assert_eq!(result, $expected, "CLI result mismatched, input: '{}'.", $args);
        };
    }

    fn mock_cli_args() -> CliArgs {
        CliArgs {
            yes: false,
            no: false,
            accessible: false,
            hidden: false,
            quiet: false,
            gitignore: false,
            format: None,
            // This is usually replaced in assertion tests
            cmd: Subcommand::Decompress {
                // Put a crazy value here so no test can assert it unintentionally
                files: vec!["\x00\x11\x22".into()],
                output_dir: None,
            },
        }
    }

    #[test]
    fn test_clap_cli_ok() {
        test!(
            "ouch decompress file.tar.gz",
            CliArgs {
                cmd: Subcommand::Decompress {
                    files: to_paths(["file.tar.gz"]),
                    output_dir: None,
                },
                ..mock_cli_args()
            }
        );
        test!(
            "ouch d file.tar.gz",
            CliArgs {
                cmd: Subcommand::Decompress {
                    files: to_paths(["file.tar.gz"]),
                    output_dir: None,
                },
                ..mock_cli_args()
            }
        );
        test!(
            "ouch d a b c",
            CliArgs {
                cmd: Subcommand::Decompress {
                    files: to_paths(["a", "b", "c"]),
                    output_dir: None,
                },
                ..mock_cli_args()
            }
        );

        test!(
            "ouch compress file file.tar.gz",
            CliArgs {
                cmd: Subcommand::Compress {
                    files: to_paths(["file"]),
                    output: PathBuf::from("file.tar.gz"),
                    level: None,
                    fast: false,
                    slow: false,
                },
                ..mock_cli_args()
            }
        );
        test!(
            "ouch compress a b c archive.tar.gz",
            CliArgs {
                cmd: Subcommand::Compress {
                    files: to_paths(["a", "b", "c"]),
                    output: PathBuf::from("archive.tar.gz"),
                    level: None,
                    fast: false,
                    slow: false,
                },
                ..mock_cli_args()
            }
        );
        test!(
            "ouch compress a b c archive.tar.gz",
            CliArgs {
                cmd: Subcommand::Compress {
                    files: to_paths(["a", "b", "c"]),
                    output: PathBuf::from("archive.tar.gz"),
                    level: None,
                    fast: false,
                    slow: false,
                },
                ..mock_cli_args()
            }
        );

        let inputs = [
            "ouch compress a b c output --format tar.gz",
            // https://github.com/clap-rs/clap/issues/5115
            // "ouch compress a b c --format tar.gz output",
            // "ouch compress a b --format tar.gz c output",
            // "ouch compress a --format tar.gz b c output",
            "ouch compress --format tar.gz a b c output",
            "ouch --format tar.gz compress a b c output",
        ];
        for input in inputs {
            test!(
                input,
                CliArgs {
                    cmd: Subcommand::Compress {
                        files: to_paths(["a", "b", "c"]),
                        output: PathBuf::from("output"),
                        level: None,
                        fast: false,
                        slow: false,
                    },
                    format: Some("tar.gz".into()),
                    ..mock_cli_args()
                }
            );
        }
    }

    #[test]
    fn test_clap_cli_err() {
        assert!(CliArgs::try_parse_from(args_splitter("ouch c")).is_err());
        assert!(CliArgs::try_parse_from(args_splitter("ouch c input")).is_err());
        assert!(CliArgs::try_parse_from(args_splitter("ouch d")).is_err());
        assert!(CliArgs::try_parse_from(args_splitter("ouch l")).is_err());
    }
}
