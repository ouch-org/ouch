use std::{
    io::{self, BufWriter, Cursor, Seek, Write},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    accessible::is_running_in_accessible_mode,
    archive,
    commands::warn_user_about_loading_zip_in_memory,
    extension::{
        split_first_compression_format,
        CompressionFormat::{self, *},
        Extension,
    },
    progress::Progress,
    utils::{user_wants_to_continue, FileVisibilityPolicy},
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};

// Compress files into an `output_file`
//
// - `files`: is the list of paths to be compressed: ["dir/file1.txt", "dir/file2.txt"]
// - `extensions`: contains each compression format necessary for compressing, example: [Tar, Gz] (in compression order)
// - `output_file` is the resulting compressed file name, example: "compressed.tar.gz"
//
// Returns Ok(true) if compressed all files successfully, and Ok(false) if user opted to skip files
pub fn compress_files(
    files: Vec<PathBuf>,
    extensions: Vec<Extension>,
    output_file: fs::File,
    output_dir: &Path,
    question_policy: QuestionPolicy,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<bool> {
    // The next lines are for displaying the progress bar
    // If the input files contain a directory, then the total size will be underestimated
    let (total_input_size, precise) = files
        .iter()
        .map(|f| (f.metadata().expect("file exists").len(), f.is_file()))
        .fold((0, true), |(total_size, and_precise), (size, precise)| {
            (total_size + size, and_precise & precise)
        });

    let file_writer = BufWriter::with_capacity(BUFFER_CAPACITY, output_file);

    let mut writer: Box<dyn Write> = Box::new(file_writer);

    // Grab previous encoder and wrap it inside of a new one
    let chain_writer_encoder = |format: &CompressionFormat, encoder: Box<dyn Write>| -> crate::Result<Box<dyn Write>> {
        let encoder: Box<dyn Write> = match format {
            Gzip => Box::new(flate2::write::GzEncoder::new(encoder, Default::default())),
            Bzip => Box::new(bzip2::write::BzEncoder::new(encoder, Default::default())),
            Lz4 => Box::new(lzzzz::lz4f::WriteCompressor::new(encoder, Default::default())?),
            Lzma => Box::new(xz2::write::XzEncoder::new(encoder, 6)),
            Snappy => Box::new(snap::write::FrameEncoder::new(encoder)),
            Zstd => {
                let zstd_encoder = zstd::stream::write::Encoder::new(encoder, Default::default());
                // Safety:
                //     Encoder::new() can only fail if `level` is invalid, but Default::default()
                //     is guaranteed to be valid
                Box::new(zstd_encoder.unwrap().auto_finish())
            }
            Tar | Zip => unreachable!(),
        };
        Ok(encoder)
    };

    let (first_format, formats) = split_first_compression_format(&extensions);

    for format in formats.iter().rev() {
        writer = chain_writer_encoder(format, writer)?;
    }

    match first_format {
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            writer = chain_writer_encoder(&first_format, writer)?;
            let mut reader = fs::File::open(&files[0]).unwrap();

            if is_running_in_accessible_mode() {
                io::copy(&mut reader, &mut writer)?;
            } else {
                io::copy(
                    &mut Progress::new(total_input_size, precise, true).wrap_read(reader),
                    &mut writer,
                )?;
            }
        }
        Tar => {
            if is_running_in_accessible_mode() {
                archive::tar::build_archive_from_paths(&files, &mut writer, file_visibility_policy, io::stderr())?;
                writer.flush()?;
            } else {
                let mut progress = Progress::new(total_input_size, precise, true);
                let mut writer = progress.wrap_write(writer);
                archive::tar::build_archive_from_paths(&files, &mut writer, file_visibility_policy, &mut progress)?;
                writer.flush()?;
            }
        }
        Zip => {
            if !formats.is_empty() {
                warn_user_about_loading_zip_in_memory();

                // give user the option to continue compressing after warning is shown
                if !user_wants_to_continue(output_dir, question_policy, QuestionAction::Compression)? {
                    return Ok(false);
                }
            }

            let mut vec_buffer = Cursor::new(vec![]);

            if is_running_in_accessible_mode() {
                archive::zip::build_archive_from_paths(&files, &mut vec_buffer, file_visibility_policy, io::stderr())?;
                vec_buffer.rewind()?;
                io::copy(&mut vec_buffer, &mut writer)?;
            } else {
                let mut progress = Progress::new(total_input_size, precise, true);
                archive::zip::build_archive_from_paths(&files, &mut vec_buffer, file_visibility_policy, &mut progress)?;
                vec_buffer.rewind()?;
                io::copy(&mut progress.wrap_read(vec_buffer), &mut writer)?;
            }
        }
    }

    Ok(true)
}
