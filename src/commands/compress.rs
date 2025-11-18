use std::{
    io::{self, BufWriter, Cursor, Seek, Write},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

use fs_err as fs;
use gzp::par::compress::{ParCompress, ParCompressBuilder};

use super::warn_user_about_loading_sevenz_in_memory;
use crate::{
    archive,
    commands::warn_user_about_loading_zip_in_memory,
    extension::{split_first_compression_format, CompressionFormat::*, Extension},
    utils::{
        io::lock_and_flush_output_stdio,
        threads::{logical_thread_count, physical_thread_count},
        user_wants_to_continue, FileVisibilityPolicy,
    },
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
#[allow(clippy::too_many_arguments)]
pub fn compress_files(
    files: Vec<PathBuf>,
    extensions: Vec<Extension>,
    output_file: fs::File,
    output_path: &Path,
    follow_symlinks: bool,
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
            Gzip => Box::new({
                // by default, ParCompress uses a default compression level of 3
                // instead of the regular default that flate2 uses
                let parz: ParCompress<gzp::deflate::Gzip, _> = ParCompressBuilder::new()
                    .compression_level(
                        level.map_or_else(Default::default, |l| gzp::Compression::new((l as u32).clamp(0, 9))),
                    )
                    .num_threads(logical_thread_count())
                    .expect("gpz: num_threads must be greater than 0")
                    .from_writer(encoder);
                parz
            }),
            Bzip => Box::new(bzip2::write::BzEncoder::new(
                encoder,
                level.map_or_else(Default::default, |l| bzip2::Compression::new((l as u32).clamp(1, 9))),
            )),
            Bzip3 => {
                #[cfg(not(feature = "bzip3"))]
                return Err(archive::bzip3_stub::no_support());

                #[cfg(feature = "bzip3")]
                Box::new(
                    // Use block size of 16 MiB
                    bzip3::write::Bz3Encoder::new(encoder, 16 * 2_usize.pow(20))?,
                )
            }
            Lz4 => Box::new(lz4_flex::frame::FrameEncoder::new(encoder).auto_finish()),
            Lzma => {
                let options = level.map_or_else(Default::default, |l| {
                    lzma_rust2::LzmaOptions::with_preset((l as u32).clamp(0, 9))
                });
                let writer = lzma_rust2::LzmaWriter::new_use_header(encoder, &options, None)?;
                Box::new(writer.auto_finish())
            }
            Xz => {
                let mut options = level.map_or_else(Default::default, |l| {
                    lzma_rust2::XzOptions::with_preset((l as u32).clamp(0, 9))
                });
                let dict_size = options.lzma_options.dict_size as u64;
                options.set_block_size(NonZeroU64::new(dict_size));
                // Use up to 256 PHYSICAL cores for compression
                let writer = lzma_rust2::XzWriterMt::new(encoder, options, physical_thread_count() as u32)?;
                Box::new(writer.auto_finish())
            }
            Lzip => {
                let options = level.map_or_else(Default::default, |l| {
                    lzma_rust2::LzipOptions::with_preset((l as u32).clamp(0, 9))
                });
                let writer = lzma_rust2::LzipWriter::new(encoder, options);
                Box::new(writer.auto_finish())
            }
            Snappy => Box::new({
                let parz: ParCompress<gzp::snap::Snap, _> = ParCompressBuilder::new()
                    .compression_level(gzp::par::compress::Compression::new(
                        level.map_or_else(Default::default, |l| (l as u32).clamp(0, 9)),
                    ))
                    .num_threads(logical_thread_count())
                    .expect("gpz: num_threads must be greater than 0")
                    .from_writer(encoder);

                parz
            }),
            Zstd => {
                let mut zstd_encoder = zstd::stream::write::Encoder::new(
                    encoder,
                    level.map_or(zstd::DEFAULT_COMPRESSION_LEVEL, |l| {
                        (l as i32).clamp(zstd::zstd_safe::min_c_level(), zstd::zstd_safe::max_c_level())
                    }),
                )?;
                // Use all available PHYSICAL cores for compression
                zstd_encoder.multithread(physical_thread_count() as u32)?;
                Box::new(zstd_encoder.auto_finish())
            }
            Brotli => {
                let default_level = 11; // Same as brotli CLI, default to highest compression
                let level = level.unwrap_or(default_level).clamp(0, 11) as u32;
                let win_size = 22; // default to 2^22 = 4 MiB window size
                Box::new(brotli::CompressorWriter::new(encoder, BUFFER_CAPACITY, level, win_size))
            }
            Tar | Zip | Rar | SevenZip => unreachable!(),
        };
        Ok(encoder)
    };

    let (first_format, formats) = split_first_compression_format(&extensions);

    for format in formats.iter().rev() {
        writer = chain_writer_encoder(format, writer)?;
    }

    match first_format {
        Gzip | Bzip | Bzip3 | Lz4 | Lzma | Xz | Lzip | Snappy | Zstd | Brotli => {
            writer = chain_writer_encoder(&first_format, writer)?;
            let mut reader = fs::File::open(&files[0])?;

            io::copy(&mut reader, &mut writer)?;
        }
        Tar => {
            archive::tar::build_archive_from_paths(
                &files,
                output_path,
                &mut writer,
                file_visibility_policy,
                follow_symlinks,
            )?;
            writer.flush()?;
        }
        Zip => {
            if !formats.is_empty() {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

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
                follow_symlinks,
            )?;
            vec_buffer.rewind()?;
            io::copy(&mut vec_buffer, &mut writer)?;
        }
        Rar => {
            #[cfg(feature = "unrar")]
            return Err(archive::rar::no_compression());

            #[cfg(not(feature = "unrar"))]
            return Err(archive::rar_stub::no_support());
        }
        SevenZip => {
            if !formats.is_empty() {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

                warn_user_about_loading_sevenz_in_memory();
                if !user_wants_to_continue(output_path, question_policy, QuestionAction::Compression)? {
                    return Ok(false);
                }
            }

            let mut vec_buffer = Cursor::new(vec![]);
            archive::sevenz::compress_sevenz(&files, output_path, &mut vec_buffer, file_visibility_policy)?;
            vec_buffer.rewind()?;
            io::copy(&mut vec_buffer, &mut writer)?;
        }
    }

    Ok(true)
}
