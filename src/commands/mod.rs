//! Receive command from the cli and call the respective function for that command.

mod compress;
mod decompress;
mod list;

use std::{ops::ControlFlow, path::PathBuf};

use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use utils::colors;

use crate::{
    check,
    cli::Subcommand,
    commands::{compress::compress_files, decompress::decompress_file, list::list_archive_contents},
    error::{Error, FinalError},
    extension::{self, build_archive_file_suggestion, parse_format},
    info,
    list::ListOptions,
    utils::{self, to_utf, EscapedPathDisplay, FileVisibilityPolicy},
    warning, CliArgs, QuestionPolicy,
};

/// Warn the user that (de)compressing this .zip archive might freeze their system.
fn warn_user_about_loading_zip_in_memory() {
    const ZIP_IN_MEMORY_LIMITATION_WARNING: &str = "\n\
        \tThe format '.zip' is limited and cannot be (de)compressed using encoding streams.\n\
        \tWhen using '.zip' with other formats, (de)compression must be done in-memory\n\
        \tCareful, you might run out of RAM if the archive is too large!";

    warning!("{}", ZIP_IN_MEMORY_LIMITATION_WARNING);
}

/// This function checks what command needs to be run and performs A LOT of ahead-of-time checks
/// to assume everything is OK.
///
/// There are a lot of custom errors to give enough error description and explanation.
pub fn run(
    args: CliArgs,
    question_policy: QuestionPolicy,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<()> {
    match args.cmd {
        Subcommand::Compress {
            files,
            output: output_path,
        } => {
            // After cleaning, if there are no input files left, exit
            if files.is_empty() {
                return Err(FinalError::with_title("No files to compress").into());
            }

            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let (formats_from_flag, formats) = match args.format {
                Some(formats) => {
                    let parsed_formats = parse_format(&formats)?;
                    (Some(formats), parsed_formats)
                }
                None => (None, extension::extensions_from_path(&output_path)),
            };

            let first_format = formats.first().ok_or_else(|| {
                let output_path = EscapedPathDisplay::new(&output_path);
                FinalError::with_title(format!("Cannot compress to '{output_path}'."))
                    .detail("You shall supply the compression format")
                    .hint("Try adding supported extensions (see --help):")
                    .hint(format!("  ouch compress <FILES>... {output_path}.tar.gz"))
                    .hint(format!("  ouch compress <FILES>... {output_path}.zip"))
                    .hint("")
                    .hint("Alternatively, you can overwrite this option by using the '--format' flag:")
                    .hint(format!("  ouch compress <FILES>... {output_path} --format tar.gz"))
            })?;

            let is_some_input_a_folder = files.iter().any(|path| path.is_dir());
            let is_multiple_inputs = files.len() > 1;

            // If first format is not archive, can't compress folder, or multiple files
            // Index safety: empty formats should be checked above.
            if !first_format.is_archive() && (is_some_input_a_folder || is_multiple_inputs) {
                let first_detail_message = if is_multiple_inputs {
                    "You are trying to compress multiple files."
                } else {
                    "You are trying to compress a folder."
                };

                let (from_hint, to_hint) = if let Some(formats) = formats_from_flag {
                    let formats = formats.to_string_lossy();
                    (
                        format!("From: --format {formats}"),
                        format!("To:   --format tar.{formats}"),
                    )
                } else {
                    // This piece of code creates a suggestion for compressing multiple files
                    // It says:
                    // Change from file.bz.xz
                    // To          file.tar.bz.xz
                    let suggested_output_path = build_archive_file_suggestion(&output_path, ".tar")
                        .expect("output path should contain a compression format");

                    (
                        format!("From: {}", EscapedPathDisplay::new(&output_path)),
                        format!("To:   {suggested_output_path}"),
                    )
                };
                let output_path = EscapedPathDisplay::new(&output_path);

                let error = FinalError::with_title(format!("Cannot compress to '{output_path}'."))
                    .detail(first_detail_message)
                    .detail(format!(
                        "The compression format '{first_format}' does not accept multiple files.",
                    ))
                    .detail("Formats that bundle files into an archive are tar and zip.")
                    .hint(format!("Try inserting 'tar.' or 'zip.' before '{first_format}'."))
                    .hint(from_hint)
                    .hint(to_hint);

                return Err(error.into());
            }

            check::check_archive_formats_position(&formats, &output_path)?;

            let output_file = match utils::ask_to_create_file(&output_path, question_policy)? {
                Some(writer) => writer,
                None => return Ok(()),
            };

            let compress_result = compress_files(
                files,
                formats,
                output_file,
                &output_path,
                args.quiet,
                question_policy,
                file_visibility_policy,
            );

            if let Ok(true) = compress_result {
                // this is only printed once, so it doesn't result in much text. On the other hand,
                // having a final status message is important especially in an accessibility context
                // as screen readers may not read a commands exit code, making it hard to reason
                // about whether the command succeeded without such a message
                info!(accessible, "Successfully compressed '{}'.", to_utf(&output_path));
            } else {
                // If Ok(false) or Err() occurred, delete incomplete file at `output_path`
                //
                // if deleting fails, print an extra alert message pointing
                // out that we left a possibly CORRUPTED file at `output_path`
                if utils::remove_file_or_dir(&output_path).is_err() {
                    eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                    eprintln!(
                        "  Ouch failed to delete the file '{}'.",
                        EscapedPathDisplay::new(&output_path)
                    );
                    eprintln!("  Please delete it manually.");
                    eprintln!("  This file is corrupted if compression didn't finished.");

                    if compress_result.is_err() {
                        eprintln!("  Compression failed for reasons below.");
                    }
                }
            }

            compress_result?;
        }
        Subcommand::Decompress { files, output_dir } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            if let Some(format) = args.format {
                let format = parse_format(&format)?;
                for path in files.iter() {
                    let file_name = path.file_name().ok_or_else(|| Error::NotFound {
                        error_title: format!("{} does not have a file name", EscapedPathDisplay::new(path)),
                    })?;
                    output_paths.push(file_name.as_ref());
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let (file_output_path, file_formats) = extension::separate_known_extensions_from_name(path);
                    output_paths.push(file_output_path);
                    formats.push(file_formats);
                }
            }

            if let ControlFlow::Break(_) = check::check_mime_type(&files, &mut formats, question_policy)? {
                return Ok(());
            }

            check::check_missing_formats_when_decompressing(&files, &formats)?;

            // The directory that will contain the output files
            // We default to the current directory if the user didn't specify an output directory with --dir
            let output_dir = if let Some(dir) = output_dir {
                utils::create_dir_if_non_existent(&dir)?;
                dir
            } else {
                PathBuf::from(".")
            };

            files
                .par_iter()
                .zip(formats)
                .zip(output_paths)
                .try_for_each(|((input_path, formats), file_name)| {
                    let output_file_path = output_dir.join(file_name); // Path used by single file format archives
                    decompress_file(
                        input_path,
                        formats,
                        &output_dir,
                        output_file_path,
                        question_policy,
                        args.quiet,
                    )
                })?;
        }
        Subcommand::List { archives: files, tree } => {
            let mut formats = vec![];

            if let Some(format) = args.format {
                let format = parse_format(&format)?;
                for _ in 0..files.len() {
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let file_formats = extension::extensions_from_path(path);
                    formats.push(file_formats);
                }

                if let ControlFlow::Break(_) = check::check_mime_type(&files, &mut formats, question_policy)? {
                    return Ok(());
                }
            }

            // Ensure we were not told to list the content of a non-archive compressed file
            check::check_for_non_archive_formats(&files, &formats)?;

            let list_options = ListOptions { tree };

            for (i, (archive_path, formats)) in files.iter().zip(formats).enumerate() {
                if i > 0 {
                    println!();
                }
                let formats = extension::flatten_compression_formats(&formats);
                list_archive_contents(archive_path, formats, list_options, question_policy)?;
            }
        }
    }
    Ok(())
}
