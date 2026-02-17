//! Checks for errors.

#![warn(missing_docs)]

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    error::FinalError,
    extension::{build_archive_file_suggestion, Extension},
    info_accessible,
    utils::{
        append_ascii_suffix_to_os_str, pretty_format_list_of_paths, try_infer_format, user_wants_to_continue, PathFmt,
    },
    warning, QuestionAction, QuestionPolicy, Result,
};

#[allow(missing_docs)]
/// Different outcomes for file signature check that the caller must handle.
pub enum CheckFileSignatureControlFlow {
    HaltProgram,
    Continue,
    ChangeToDetectedExtension {
        new_extension: Extension,
        new_path_filename: OsString,
    },
}

/// Check if the file signature matches the detected extensions.
///
/// If the path didn't have any extensions, try to infer the format from signature.
///
/// Note that Brotli can't be detected by signature.
///
/// # Panics
///
/// - Panics if `path` has no filename.
pub fn check_file_signature(
    path: &Path,
    extensions: &[Extension],
    question_policy: QuestionPolicy,
) -> Result<CheckFileSignatureControlFlow> {
    debug_assert!(path.file_name().is_some());

    let detected_format = try_infer_format(path);
    let outer_format_from_path = extensions
        .last()
        .and_then(|extension| extension.compression_formats.last())
        .copied();

    match (detected_format, outer_format_from_path) {
        (None, None) => {
            // Do nothing, so these cases will be reported at `check::check_missing_formats_when_decompressing` together
        }
        (None, Some(_from_path)) => {
            // TODO: promote to a warning and ask the user to proceed
            info_accessible!(
                "Failed to confirm the format of {:?} by sniffing the contents, file might be misnamed",
                PathFmt(path),
            );
        }
        (Some(detected), None) => {
            warning!(
                "No recognized extensions in {:?}. Proceeding with `{}` that was detected from the file signature.",
                PathFmt(path),
                detected.as_str(),
            );

            // TODO: change question to: "do you want to proceed regardless of that"?
            if !user_wants_to_continue(path, question_policy, QuestionAction::Decompression)? {
                return Ok(CheckFileSignatureControlFlow::HaltProgram);
            }

            // We usually get the output path name by removing the extensions, in this scenario
            // we didn't recognized path extensions, so we need to improvise to create a
            // reasonable output path name
            let new_path_filename =
                append_ascii_suffix_to_os_str(path.with_extension("").file_name().unwrap(), "-output");
            return Ok(CheckFileSignatureControlFlow::ChangeToDetectedExtension {
                new_path_filename,
                new_extension: Extension::from_format(detected),
            });
        }
        (Some(detected), Some(from_path)) => {
            if from_path != detected {
                let error = FinalError::with_title(format!("Format mismatch for {:?}", PathFmt(path)))
                    .detail(format!(
                        "File extension suggests `{}`, but file signature indicates `{}`",
                        from_path.as_str(),
                        detected.as_str(),
                    ))
                    .hint(format!(
                        "Use the `--format {}` flag to specify the correct format",
                        detected.as_str()
                    ))
                    .hint("(If that's not correct, please rename the file)");

                return Err(error.into());
            }
        }
    }

    Ok(CheckFileSignatureControlFlow::Continue)
}

/// In the context of listing archives, this function checks if `ouch` was told to list
/// the contents of a compressed file that is not an archive
pub fn check_for_non_archive_formats(files: &[PathBuf], formats: &[Vec<Extension>]) -> Result<()> {
    let mut not_archives = files
        .iter()
        .zip(formats)
        .filter(|(_, formats)| !formats.first().map(Extension::is_archive).unwrap_or(false))
        .map(|(path, _)| path)
        .peekable();

    if not_archives.peek().is_some() {
        let not_archives: Vec<_> = not_archives.collect();
        let error = FinalError::with_title("Cannot list archive contents")
            .detail("Only archives can have their contents listed")
            .detail(format!(
                "Files are not archives: {}",
                pretty_format_list_of_paths(&not_archives)
            ));

        return Err(error.into());
    }

    Ok(())
}

/// Show error if archive format is not the first format in the chain.
pub fn check_archive_formats_position(formats: &[Extension], output_path: &Path) -> Result<()> {
    if let Some(format) = formats.iter().skip(1).find(|format| format.is_archive()) {
        let error = FinalError::with_title(format!("Cannot compress to {:?}", PathFmt(output_path)))
            .detail(format!("Found the format '{format}' in an incorrect position."))
            .detail(format!(
                "'{format}' can only be used at the start of the file extension."
            ))
            .hint(format!(
                "If you wish to compress multiple files, start the extension with '{format}'."
            ))
            .hint(format!(
                "Otherwise, remove the last '{}' from {:?}.",
                format,
                PathFmt(output_path)
            ));

        return Err(error.into());
    }
    Ok(())
}

/// Check if all provided files have formats to decompress.
pub fn check_missing_formats_when_decompressing(files: &[PathBuf], formats: &[Vec<Extension>]) -> Result<()> {
    let files_with_broken_extension: Vec<&PathBuf> = files
        .iter()
        .zip(formats)
        .filter(|(_, format)| format.is_empty())
        .map(|(input_path, _)| input_path)
        .collect();

    if files_with_broken_extension.is_empty() {
        return Ok(());
    }

    let (files_with_unsupported_extensions, files_missing_extension): (Vec<&PathBuf>, Vec<&PathBuf>) =
        files_with_broken_extension
            .iter()
            .partition(|path| path.extension().is_some());

    let mut error = FinalError::with_title("Cannot decompress files");

    if !files_with_unsupported_extensions.is_empty() {
        error = error.detail(format!(
            "Files with unsupported extensions: {}",
            pretty_format_list_of_paths(&files_with_unsupported_extensions)
        ));
    }

    if !files_missing_extension.is_empty() {
        error = error.detail(format!(
            "Files with missing extensions: {}",
            pretty_format_list_of_paths(&files_missing_extension)
        ));
    }

    error = error.detail("Decompression formats are detected automatically from file extension and signature");
    error = error.hint_all_supported_formats();

    // If there's exactly one file, give a suggestion to use `--format`
    if let &[path] = files_with_broken_extension.as_slice() {
        error = error
            .hint("")
            .hint("Alternatively, you can pass an extension to the '--format' flag:")
            .hint(format!("  ouch decompress {} --format tar.gz", PathFmt(path)));
    }

    Err(error.into())
}

/// Check if there is a first format when compressing, and returns it.
pub fn check_first_format_when_compressing<'a>(formats: &'a [Extension], output_path: &Path) -> Result<&'a Extension> {
    formats.first().ok_or_else(|| {
        FinalError::with_title(format!("Cannot compress to {:?}", PathFmt(output_path)))
            .detail("You must supply the compression format")
            .hint("Try adding supported extensions (see --help):")
            .hint(format!("  ouch compress <FILES>... {}.tar.gz", PathFmt(output_path)))
            .hint(format!("  ouch compress <FILES>... {}.zip", PathFmt(output_path)))
            .hint("")
            .hint("Alternatively, you can overwrite this option by using the '--format' flag:")
            .hint(format!(
                "  ouch compress <FILES>... {} --format tar.gz",
                PathFmt(output_path),
            ))
            .into()
    })
}

/// Check if compression is invalid because an archive format is necessary.
///
/// Non-archive formats don't support multiple file compression or folder compression.
pub fn check_invalid_compression_with_non_archive_format(
    formats: &[Extension],
    output_path: &Path,
    files: &[PathBuf],
    formats_from_flag: Option<&str>,
) -> Result<()> {
    let first_format = check_first_format_when_compressing(formats, output_path)?;

    let is_some_input_a_folder = files.iter().any(|path| path.is_dir());
    let is_multiple_inputs = files.len() > 1;

    // If format is archive, nothing to check
    // If there's no folder or multiple inputs, non-archive formats can handle it
    if first_format.is_archive() || !is_some_input_a_folder && !is_multiple_inputs {
        return Ok(());
    }

    let first_detail_message = if is_multiple_inputs {
        "You are trying to compress multiple files."
    } else {
        "You are trying to compress a folder."
    };

    let (from_hint, to_hint) = if let Some(formats) = formats_from_flag {
        (
            format!("From: --format {formats}"),
            format!("To:   --format tar.{formats}"),
        )
    } else {
        // This piece of code creates a suggestion for compressing multiple files
        // It says:
        // Change from file.bz.xz
        // To          file.tar.bz.xz
        let suggested_output_path = build_archive_file_suggestion(output_path, ".tar")
            .expect("output path should contain a compression format");

        (
            format!("From: {:?}", PathFmt(output_path)),
            format!("To:   \"{suggested_output_path}\""),
        )
    };

    let error = FinalError::with_title(format!("Cannot compress to {:?}", PathFmt(output_path)))
        .detail(first_detail_message)
        .detail(format!(
            "The compression format '{first_format}' does not accept multiple files.",
        ))
        .detail("Formats that bundle files into an archive are tar, zip and 7z.")
        .hint(format!(
            "Try inserting 'tar.', 'zip.' or '7z.' before '{first_format}'."
        ))
        .hint(from_hint)
        .hint(to_hint);

    Err(error.into())
}
