// Public modules
pub mod cli;
pub mod commands;
pub mod oof;

// Private modules
pub mod archive;
mod dialogs;
mod error;
mod extension;
mod macros;
mod utils;

pub use error::{Error, Result};

pub const EXIT_FAILURE: i32 = 127;

const VERSION: &str = "0.1.5";

fn help_command() {
    use utils::colors::*;
    /*
    ouch - Obvious Unified Compressed files Helper

    USAGE:
        ouch <files...>                        Decompresses files.

        ouch compress <files...> OUTPUT.EXT    Compresses files into OUTPUT.EXT,
                                               where EXT must be a supported format.

    FLAGS:
        -h, --help    Display this help information.
        -y, --yes     Skip overwrite questions.
        -n, --no      Skip overwrite questions.
        --version     Display version information.

    SPECIFIC FLAGS:
        -o, --output FOLDER_PATH    When decompressing, to decompress files to
                                    another folder.

    Visit https://github.com/vrmiguel/ouch for more usage examples.
    */

    println!(
        "\
{cyan}ouch{reset} - Obvious Unified Compression files Helper

{cyan}USAGE:{reset}
    {green}ouch {magenta}<files...>{reset}                        Decompresses files.

    {green}ouch compress {magenta}<files...> OUTPUT.EXT{reset}    Compresses files into {magenta}OUTPUT.EXT{reset},
                                           where {magenta}EXT{reset} must be a supported format.

{cyan}FLAGS:{reset}
    {yellow}-h{white}, {yellow}--help{reset}    Display this help information.
    {yellow}-y{white}, {yellow}--yes{reset}     Skip overwrite questions.
    {yellow}-n{white}, {yellow}--no{reset}      Skip overwrite questions.
    {yellow}--version{reset}     Display version information.

{cyan}SPECIFIC FLAGS:{reset}
    {yellow}-o{reset}, {yellow}--output{reset} FOLDER_PATH    When decompressing, to decompress files to
                                another folder.

Visit https://github.com/vrmiguel/ouch for more usage examples.",
        magenta = magenta(),
        white = white(),
        green = green(),
        yellow = yellow(),
        reset = reset(),
        cyan = cyan()
    );
}

#[inline]
fn version_command() {
    use utils::colors::*;
    println!("{green}ouch{reset} {}", crate::VERSION, green = green(), reset = reset());
}
