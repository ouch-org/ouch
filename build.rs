use std::{env, fs::create_dir_all, path::Path};

use clap::{ArgEnum, IntoApp};
use clap_complete::{generate_to, Shell};

include!("src/opts.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=GEN_COMPLETIONS");

    if env::var_os("GEN_COMPLETIONS") != Some("1".into()) {
        return;
    }

    let out = &Path::new(&env::var_os("OUT_DIR").unwrap()).join("completions");
    create_dir_all(out).unwrap();
    let app = &mut Opts::into_app();

    for shell in Shell::value_variants() {
        generate_to(*shell, app, "ouch", out).unwrap();
    }
}
