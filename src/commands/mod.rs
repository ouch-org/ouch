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
    extension::{self, parse_format_flag},
    info,
    list::ListOptions,
    utils::{self, path_to_str, FileVisibilityPolicy},
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

/// Warn the user that (de)compressing this .7z archive might freeze their system.
fn warn_user_about_loading_sevenz_in_memory() {
    const SEVENZ_IN_MEMORY_LIMITATION_WARNING: &str = "\n\
        \tThe format '.7z' is limited and cannot be (de)compressed using encoding streams.\n\
        \tWhen using '.7z' with other formats, (de)compression must be done in-memory\n\
        \tCareful, you might run out of RAM if the archive is too large!";

    warning!("{}", SEVENZ_IN_MEMORY_LIMITATION_WARNING);
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
            level,
            fast,
            slow,
        } => {
            // After cleaning, if there are no input files left, exit
            if files.is_empty() {
                return Err(FinalError::with_title("No files to compress").into());
            }

            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let (formats_from_flag, formats) = match args.format {
                Some(formats) => {
                    let parsed_formats = parse_format_flag(&formats)?;
                    (Some(formats), parsed_formats)
                }
                None => (None, extension::extensions_from_path(&output_path)),
            };

            check::check_invalid_compression_with_non_archive_format(
                &formats,
                &output_path,
                &files,
                formats_from_flag.as_ref(),
            )?;
            check::check_archive_formats_position(&formats, &output_path)?;

            let output_file = match utils::ask_to_create_file(&output_path, question_policy)? {
                Some(writer) => writer,
                None => return Ok(()),
            };

            let level = if fast {
                Some(1) // Lowest level of compression
            } else if slow {
                Some(i16::MAX) // Highest level of compression
            } else {
                level
            };

            let compress_result = compress_files(
                files,
                formats,
                output_file,
                &output_path,
                args.quiet,
                question_policy,
                file_visibility_policy,
                level,
            );

            if let Ok(true) = compress_result {
                // this is only printed once, so it doesn't result in much text. On the other hand,
                // having a final status message is important especially in an accessibility context
                // as screen readers may not read a commands exit code, making it hard to reason
                // about whether the command succeeded without such a message
                info!(accessible, "Successfully compressed '{}'.", path_to_str(&output_path));
            } else {
                // If Ok(false) or Err() occurred, delete incomplete file at `output_path`
                //
                // if deleting fails, print an extra alert message pointing
                // out that we left a possibly CORRUPTED file at `output_path`
                if utils::remove_file_or_dir(&output_path).is_err() {
                    eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                    eprintln!("  Ouch failed to delete the file '{}'.", &output_path.display());
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
                let format = parse_format_flag(&format)?;
                for path in files.iter() {
                    let file_name = path.file_name().ok_or_else(|| Error::NotFound {
                        error_title: format!("{} does not have a file name", path.display()),
                    })?;
                    output_paths.push(file_name.as_ref());
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let (pathbase, mut file_formats) = extension::separate_known_extensions_from_name(path);

                    if let ControlFlow::Break(_) = check::check_mime_type(path, &mut file_formats, question_policy)? {
                        return Ok(());
                    }

                    output_paths.push(pathbase);
                    formats.push(file_formats);
                }
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
                let format = parse_format_flag(&format)?;
                for _ in 0..files.len() {
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let mut file_formats = extension::extensions_from_path(path);

                    if let ControlFlow::Break(_) = check::check_mime_type(path, &mut file_formats, question_policy)? {
                        return Ok(());
                    }

                    formats.push(file_formats);
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
