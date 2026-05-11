/// This build script checks for env vars to build ouch with shell completions and man pages.
///
/// # How to generate shell completions and man pages:
///
/// Set `OUCH_ARTIFACTS_FOLDER` to the name of the destination folder:
///
/// ```sh
/// OUCH_ARTIFACTS_FOLDER=man-page-and-completions-artifacts cargo build
/// ```
///
/// All completion files will be generated inside of the folder "man-page-and-completions-artifacts".
///
/// If the folder does not exist, it will be created.
use std::{env, fs::create_dir_all, path::Path};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{Shell, generate_to};
use clap_complete_nushell::Nushell;

include!("src/cli/args.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=OUCH_ARTIFACTS_FOLDER");

    if let Some(dir) = env::var_os("OUCH_ARTIFACTS_FOLDER") {
        let out = &Path::new(&dir);
        create_dir_all(out).unwrap();
        let cmd = &mut CliArgs::command();

        clap_mangen::generate_to(cmd.clone(), out).unwrap();

        for shell in Shell::value_variants() {
            generate_to(*shell, cmd, "ouch", out).unwrap();
        }
        generate_to(Nushell, cmd, "ouch", out).unwrap();
    }
}
