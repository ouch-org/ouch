use std::{env, fs::create_dir_all};

use clap::{ArgEnum, IntoApp};
use clap_complete::{generate_to, Shell};

include!("src/opts.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=GEN_COMPLETIONS");

    if env::var_os("GEN_COMPLETIONS") != Some("1".into()) {
        return;
    }

    let out = &env::var_os("OUCH_COMPLETIONS_DIR")
        .map(|path| PathBuf::from(&path))
        .or_else(|| env::var_os("OUT_DIR").map(|path| PathBuf::from(&path)).map(|path| path.join("completions")))
        .unwrap();

    create_dir_all(out).unwrap();
    let app = &mut Opts::into_app();

    for shell in Shell::value_variants() {
        generate_to(*shell, app, "ouch", out).unwrap();
    }
}
