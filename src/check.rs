//! Checks for errors.

#![warn(missing_docs)]

use std::{
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use crate::{
    error::FinalError,
    extension,
    extension::Extension,
    info,
    utils::{pretty_format_list_of_paths, try_infer_extension, user_wants_to_continue, EscapedPathDisplay},
    warning, QuestionAction, QuestionPolicy, Result,
};

/// Check, for each file, if the mime type matches the detected extensions.
///
/// In case the file doesn't has any extensions, try to infer the format.
///
/// TODO: maybe the name of this should be "magic numbers" or "file signature",
/// and not MIME.
pub fn check_mime_type(
    files: &[PathBuf],
    formats: &mut [Vec<Extension>],
    question_policy: QuestionPolicy,
) -> Result<ControlFlow<()>> {
    for (path, format) in files.iter().zip(formats.iter_mut()) {
        if format.is_empty() {
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
                    format.push(detected_format);
                } else {
                    return Ok(ControlFlow::Break(()));
                }
            }
        } else if let Some(detected_format) = try_infer_extension(path) {
            // File ending with extension
            // Try to detect the extension and warn the user if it differs from the written one
            let outer_ext = format.iter().next_back().unwrap();
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
            info!(accessible, "Could not detect the extension of `{}`", path.display());
        }
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
pub fn check_archive_formats_position(formats: &[extension::Extension], output_path: &Path) -> Result<()> {
    if let Some(format) = formats.iter().skip(1).find(|format| format.is_archive()) {
        let error = FinalError::with_title(format!(
            "Cannot compress to '{}'.",
            EscapedPathDisplay::new(output_path)
        ))
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
            EscapedPathDisplay::new(output_path)
        ));

        return Err(error.into());
    }
    Ok(())
}
