use std::{
    io::{self, BufWriter, Cursor, Seek, Write},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    archive,
    commands::warn_user_about_loading_zip_in_memory,
    extension::{
        split_first_compression_format,
        CompressionFormat::{self, *},
        Extension,
    },
    utils::{user_wants_to_continue, FileVisibilityPolicy},
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};

/// Compress files into `output_file`.
///
/// # Arguments:
/// - `files`: is the list of paths to be compressed: ["dir/file1.txt", "dir/file2.txt"]
/// - `extensions`: is a list of compression formats for compressing, example: [Tar, Gz] (in compression order)
/// - `output_file` is the resulting compressed file name, example: "archive.tar.gz"
///
/// # Return value
/// - Returns `Ok(true)` if compressed all files normally.
/// - Returns `Ok(false)` if user opted to abort compression mid-way.
pub fn compress_files(
    files: Vec<PathBuf>,
    extensions: Vec<Extension>,
    output_file: fs::File,
    output_path: &Path,
    quiet: bool,
    question_policy: QuestionPolicy,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<bool> {
    // If the input files contain a directory, then the total size will be underestimated
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

            io::copy(&mut reader, &mut writer)?;
        }
        Tar => {
            archive::tar::build_archive_from_paths(&files, output_path, &mut writer, quiet, file_visibility_policy)?;
            writer.flush()?;
        }
        Zip => {
            if !formats.is_empty() {
                warn_user_about_loading_zip_in_memory();

                if !user_wants_to_continue(output_path, question_policy, QuestionAction::Compression)? {
                    return Ok(false);
                }
            }

            let mut vec_buffer = Cursor::new(vec![]);

            archive::zip::build_archive_from_paths(&files, output_path, &mut vec_buffer, quiet, file_visibility_policy)?;
            vec_buffer.rewind()?;
            io::copy(&mut vec_buffer, &mut writer)?;
        }
    }

    Ok(true)
}
