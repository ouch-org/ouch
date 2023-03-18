use std::{
    io::{self, BufWriter, Cursor, Seek, Write},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    archive,
    commands::warn_user_about_loading_zip_in_memory,
    extension::{split_first_compression_format, CompressionFormat::*, Extension},
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
    level: Option<i16>,
) -> crate::Result<bool> {
    // If the input files contain a directory, then the total size will be underestimated
    let file_writer = BufWriter::with_capacity(BUFFER_CAPACITY, output_file);

    let mut writer: Box<dyn Send + Write> = Box::new(file_writer);

    // Grab previous encoder and wrap it inside of a new one
    let chain_writer_encoder = |format: &_, encoder| -> crate::Result<_> {
        let encoder: Box<dyn Send + Write> = match format {
            Gzip => Box::new(
                // by default, ParCompress uses a default compression level of 3
                // instead of the regular default that flate2 uses
                gzp::par::compress::ParCompress::<gzp::deflate::Gzip>::builder()
                    .compression_level(
                        level.map_or_else(Default::default, |l| gzp::Compression::new((l as u32).clamp(0, 9))),
                    )
                    .from_writer(encoder),
            ),
            Bzip => Box::new(bzip2::write::BzEncoder::new(
                encoder,
                level.map_or_else(Default::default, |l| bzip2::Compression::new((l as u32).clamp(1, 9))),
            )),
            Lz4 => Box::new(lzzzz::lz4f::WriteCompressor::new(
                encoder,
                lzzzz::lz4f::PreferencesBuilder::new()
                    .compression_level(level.map_or(0, |l| (l as i32).clamp(1, lzzzz::lz4f::CLEVEL_MAX)))
                    .build(),
            )?),
            Lzma => Box::new(xz2::write::XzEncoder::new(
                encoder,
                level.map_or(6, |l| (l as u32).clamp(0, 9)),
            )),
            Snappy => Box::new(
                gzp::par::compress::ParCompress::<gzp::snap::Snap>::builder()
                    .compression_level(gzp::par::compress::Compression::new(
                        level.map_or_else(Default::default, |l| (l as u32).clamp(0, 9)),
                    ))
                    .from_writer(encoder),
            ),
            Zstd => Box::new(
                zstd::stream::write::Encoder::new(
                    encoder,
                    level.map_or(0, |l| {
                        (l as i32).clamp(zstd::zstd_safe::min_c_level(), zstd::zstd_safe::max_c_level())
                    }),
                )
                .unwrap()
                .auto_finish(),
            ),
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
            archive::tar::build_archive_from_paths(&files, output_path, &mut writer, file_visibility_policy, quiet)?;
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

            archive::zip::build_archive_from_paths(
                &files,
                output_path,
                &mut vec_buffer,
                file_visibility_policy,
                quiet,
            )?;
            vec_buffer.rewind()?;
            io::copy(&mut vec_buffer, &mut writer)?;
        }
    }

    Ok(true)
}
