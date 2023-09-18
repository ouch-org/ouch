//! Checks for errors.

#![warn(missing_docs)]

use std::{
    ffi::OsString,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use crate::{
    error::FinalError,
    extension::{build_archive_file_suggestion, Extension},
    info,
    utils::{pretty_format_list_of_paths, try_infer_extension, user_wants_to_continue},
    warning, QuestionAction, QuestionPolicy, Result,
};

/// Check if the mime type matches the detected extensions.
///
/// In case the file doesn't has any extensions, try to infer the format.
///
/// TODO: maybe the name of this should be "magic numbers" or "file signature",
/// and not MIME.
pub fn check_mime_type(
    path: &Path,
    formats: &mut Vec<Extension>,
    question_policy: QuestionPolicy,
) -> Result<ControlFlow<()>> {
    if formats.is_empty() {
        // File with no extension
        // Try to detect it automatically and prompt the user about it
        if let Some(detected_format) = try_infer_extension(path) {
            // Inferring the file extension can have unpredicted consequences (e.g. the user just
            // mistyped, ...) which we should always inform the user about.
            info!(
                accessible,
                "Detected file: `{}` extension as `{}`",
                path.display(),
                detected_format
            );
            if user_wants_to_continue(path, question_policy, QuestionAction::Decompression)? {
                formats.push(detected_format);
            } else {
                return Ok(ControlFlow::Break(()));
            }
        }
    } else if let Some(detected_format) = try_infer_extension(path) {
        // File ending with extension
        // Try to detect the extension and warn the user if it differs from the written one

        let outer_ext = formats.iter().next_back().unwrap();
        if !outer_ext
            .compression_formats
            .ends_with(detected_format.compression_formats)
        {
            warning!(
                "The file extension: `{}` differ from the detected extension: `{}`",
                outer_ext,
                detected_format
            );
            if !user_wants_to_continue(path, question_policy, QuestionAction::Decompression)? {
                return Ok(ControlFlow::Break(()));
            }
        }
    } else {
        // NOTE: If this actually produces no false positives, we can upgrade it in the future
        // to a warning and ask the user if he wants to continue decompressing.
        info!(
            accessible,
            "Failed to confirm the format of `{}` by sniffing the contents, file might be misnamed",
            path.display()
        );
    }
    Ok(ControlFlow::Continue(()))
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
        let error = FinalError::with_title(format!("Cannot compress to '{}'.", output_path.display()))
            .detail(format!("Found the format '{format}' in an incorrect position."))
            .detail(format!(
                "'{format}' can only be used at the start of the file extension."
            ))
            .hint(format!(
                "If you wish to compress multiple files, start the extension with '{format}'."
            ))
            .hint(format!(
                "Otherwise, remove the last '{}' from '{}'.",
                format,
                output_path.display()
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

    error = error.detail("Decompression formats are detected automatically from file extension");
    error = error.hint_all_supported_formats();

    // If there's exactly one file, give a suggestion to use `--format`
    if let &[path] = files_with_broken_extension.as_slice() {
        error = error
            .hint("")
            .hint("Alternatively, you can pass an extension to the '--format' flag:")
            .hint(format!("  ouch decompress {} --format tar.gz", path.display(),));
    }

    Err(error.into())
}

/// Check if there is a first format when compressing, and returns it.
pub fn check_first_format_when_compressing<'a>(formats: &'a [Extension], output_path: &Path) -> Result<&'a Extension> {
    formats.first().ok_or_else(|| {
        let output_path = output_path.display();
        FinalError::with_title(format!("Cannot compress to '{output_path}'."))
            .detail("You shall supply the compression format")
            .hint("Try adding supported extensions (see --help):")
            .hint(format!("  ouch compress <FILES>... {output_path}.tar.gz"))
            .hint(format!("  ouch compress <FILES>... {output_path}.zip"))
            .hint("")
            .hint("Alternatively, you can overwrite this option by using the '--format' flag:")
            .hint(format!("  ouch compress <FILES>... {output_path} --format tar.gz"))
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
    formats_from_flag: Option<&OsString>,
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
        let suggested_output_path = build_archive_file_suggestion(output_path, ".tar")
            .expect("output path should contain a compression format");

        (
            format!("From: {}", output_path.display()),
            format!("To:   {suggested_output_path}"),
        )
    };
    let output_path = output_path.display();

    let error = FinalError::with_title(format!("Cannot compress to '{output_path}'."))
        .detail(first_detail_message)
        .detail(format!(
            "The compression format '{first_format}' does not accept multiple files.",
        ))
        .detail("Formats that bundle files into an archive are tar and zip.")
        .hint(format!("Try inserting 'tar.' or 'zip.' before '{first_format}'."))
        .hint(from_hint)
        .hint(to_hint);

    Err(error.into())
}
