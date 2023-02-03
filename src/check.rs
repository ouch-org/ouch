use std::{ops::ControlFlow, path::PathBuf};

use crate::{
    extension::Extension,
    info,
    utils::{try_infer_extension, user_wants_to_continue},
    warning, QuestionAction, QuestionPolicy,
};

pub fn check_mime_type(
    files: &[PathBuf],
    formats: &mut [Vec<Extension>],
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<()>> {
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
