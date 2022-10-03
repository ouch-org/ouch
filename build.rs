/// This build script checks for env vars to build ouch with shell completions and man pages.
///
/// # How to generate shell completions and man pages:
///
/// Set `OUCH_ARTIFACTS_FOLDER` to the name of the destination folder:
///
/// ```sh
/// OUCH_ARTIFACTS_FOLDER=my-folder cargo build
/// ```
///
/// All completion files will be generated inside of the folder "my-folder".
///
/// If the folder does not exist, it will be created.
///
/// We recommend you naming this folder "artifacts" for the sake of consistency.
///
/// ```sh
/// OUCH_ARTIFACTS_FOLDER=artifacts cargo build
/// ```
use std::{
    env,
    fs::{create_dir_all, File},
    path::Path,
};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;

include!("src/opts.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=OUCH_ARTIFACTS_FOLDER");

    if let Some(dir) = env::var_os("OUCH_ARTIFACTS_FOLDER") {
        let out = &Path::new(&dir);
        create_dir_all(out).unwrap();
        let cmd = &mut Opts::command();

        Man::new(cmd.clone())
            .render(&mut File::create(out.join("ouch.1")).unwrap())
            .unwrap();

        for subcmd in cmd.get_subcommands() {
            let name = format!("ouch-{}", subcmd.get_name());
            Man::new(subcmd.clone().name(&name))
                .render(&mut File::create(out.join(format!("{name}.1"))).unwrap())
                .unwrap();
        }

        for shell in Shell::value_variants() {
            generate_to(*shell, cmd, "ouch", out).unwrap();
        }
    }
}
