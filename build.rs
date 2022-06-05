/// This build script checks for env vars to build ouch with shell completions.
///
/// # How to generate shell completions:
///
/// Set `OUCH_COMPLETIONS_FOLDER` to the name of the destination folder:
///
/// ```sh
/// OUCH_COMPLETIONS_FOLDER=my-folder cargo build
/// ```
///
/// All completion files will be generated inside of the folder "my-folder".
///
/// If the folder does not exist, it will be created.
///
/// We recommend you naming this folder "completions" for the sake of consistency.
///
/// ```sh
/// OUCH_COMPLETIONS_FOLDER=completions cargo build
/// ```
///
/// # Retrocompatibility
///
/// The old method that still works so it does not break older packages.
///
/// Using `GEN_COMPLETIONS=1` still works for those packages who need it,
/// however.
///
/// ```sh
/// GEN_COMPLETIONS=1 cargo build
/// ```
///
/// Will generate completions to a cargo target default folder, for example:
/// - `target/debug/build/ouch-195b34a8adca6ec3/out/completions`
///
/// The _"195b34a8adca6ec3"_ part is a hash that might change between runs.
use std::{env, fs::create_dir_all, path::Path};

use clap::{ArgEnum, IntoApp};
use clap_complete::{generate_to, Shell};

include!("src/opts.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=GEN_COMPLETIONS");
    println!("cargo:rerun-if-env-changed=OUCH_COMPLETIONS_FOLDER");

    if let Some(completions_output_directory) = detect_completions_output_directory() {
        create_dir_all(&completions_output_directory).expect("Could not create shell completions output folder.");
        let app = &mut Opts::command();

        for shell in Shell::value_variants() {
            generate_to(*shell, app, "ouch", &completions_output_directory)
                .unwrap_or_else(|err| panic!("Failed to generate shell completions for {}: {}.", shell, err));
        }
    }
}

/// Decide whether or not to generate completions, and the destination.
///
/// Note that `OUCH_COMPLETIONS_FOLDER` is checked before `GEN_COMPLETIONS`.
fn detect_completions_output_directory() -> Option<PathBuf> {
    // Get directory from var
    if let Some(dir) = env::var_os("OUCH_COMPLETIONS_FOLDER") {
        return Some(dir.into());
    };

    // If set, directory goes inside of cargo's `target/`
    let gen_completions = env::var_os("GEN_COMPLETIONS").map(|var| &var == "1").unwrap_or(false);
    if gen_completions {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dir = Path::new(&out_dir).join("completions");
        Some(dir)
    } else {
        None
    }
}
