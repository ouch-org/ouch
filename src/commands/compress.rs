use std::{
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    archive,
    commands::warn_user_about_in_memory_zip_compression,
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
// files are the list of paths to be compressed: ["dir/file1.txt", "dir/file2.txt"]
// formats contains each format necessary for compression, example: [Tar, Gz] (in compression order)
// output_file is the resulting compressed file name, example: "compressed.tar.gz"
//
// Returns Ok(true) if compressed all files successfully, and Ok(false) if user opted to skip files
pub fn compress_files(
    files: Vec<PathBuf>,
    formats: Vec<Extension>,
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

    // NOTE: canonicalize is here to avoid a weird bug:
    //      > If output_file_path is a nested path and it exists and the user overwrite it
    //      >> output_file_path.exists() will always return false (somehow)
    //      - canonicalize seems to fix this
    let output_file_path = output_file.path().canonicalize()?;

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

    let (first_extension, extensions) = split_first_compression_format(&formats);

    for format in extensions.iter().rev() {
        writer = chain_writer_encoder(format, writer)?;
    }

    match first_extension {
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            let _progress = Progress::new_accessible_aware(
                total_input_size,
                precise,
                Some(Box::new(move || {
                    output_file_path.metadata().expect("file exists").len()
                })),
            );

            writer = chain_writer_encoder(&first_extension, writer)?;
            let mut reader = fs::File::open(&files[0]).unwrap();
            io::copy(&mut reader, &mut writer)?;
        }
        Tar => {
            let mut progress = Progress::new_accessible_aware(
                total_input_size,
                precise,
                Some(Box::new(move || {
                    output_file_path.metadata().expect("file exists").len()
                })),
            );

            archive::tar::build_archive_from_paths(
                &files,
                &mut writer,
                file_visibility_policy,
                progress
                    .as_mut()
                    .map(Progress::display_handle)
                    .unwrap_or(&mut io::stdout()),
            )?;
            writer.flush()?;
        }
        Zip => {
            if formats.len() > 1 {
                warn_user_about_in_memory_zip_compression();

                // give user the option to continue compressing after warning is shown
                if !user_wants_to_continue(output_dir, question_policy, QuestionAction::Compression)? {
                    return Ok(false);
                }
            }

            let mut vec_buffer = io::Cursor::new(vec![]);

            let current_position_fn = {
                let vec_buffer_ptr = {
                    struct FlyPtr(*const io::Cursor<Vec<u8>>);
                    unsafe impl Send for FlyPtr {}
                    FlyPtr(&vec_buffer as *const _)
                };
                Box::new(move || {
                    let vec_buffer_ptr = &vec_buffer_ptr;
                    // Safety: ptr is valid and vec_buffer is still alive
                    unsafe { &*vec_buffer_ptr.0 }.position()
                })
            };

            let mut progress = Progress::new_accessible_aware(total_input_size, precise, Some(current_position_fn));

            archive::zip::build_archive_from_paths(
                &files,
                &mut vec_buffer,
                file_visibility_policy,
                progress
                    .as_mut()
                    .map(Progress::display_handle)
                    .unwrap_or(&mut io::stdout()),
            )?;
            let vec_buffer = vec_buffer.into_inner();
            io::copy(&mut vec_buffer.as_slice(), &mut writer)?;
        }
    }

    Ok(true)
}
