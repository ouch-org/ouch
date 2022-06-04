use std::{
    io::{self, BufReader, Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    commands::warn_user_about_in_memory_zip_decompression,
    extension::{
        split_first_compression_format,
        CompressionFormat::{self, *},
        Extension,
    },
    info,
    progress::Progress,
    utils::{self, nice_directory_display, user_wants_to_continue},
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};

// Decompress a file
//
// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
// output_dir it's where the file will be decompressed to, this function assumes that the directory exists
// output_file_path is only used when extracting single file formats, not archive formats like .tar or .zip
pub fn decompress_file(
    input_file_path: &Path,
    formats: Vec<Extension>,
    output_dir: &Path,
    output_file_path: PathBuf,
    question_policy: QuestionPolicy,
) -> crate::Result<()> {
    assert!(output_dir.exists());
    let total_input_size = input_file_path.metadata().expect("file exists").len();
    let reader = fs::File::open(&input_file_path)?;

    // Zip archives are special, because they require io::Seek, so it requires it's logic separated
    // from decoder chaining.
    //
    // This is the only case where we can read and unpack it directly, without having to do
    // in-memory decompression/copying first.
    //
    // Any other Zip decompression done can take up the whole RAM and freeze ouch.
    if let [Extension {
        compression_formats: [Zip],
        ..
    }] = formats.as_slice()
    {
        let zip_archive = zip::ZipArchive::new(reader)?;
        let files = if let ControlFlow::Continue(files) = smart_unpack(
            Box::new(move |output_dir| {
                let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                crate::archive::zip::unpack_archive(
                    zip_archive,
                    output_dir,
                    progress
                        .as_mut()
                        .map(Progress::display_handle)
                        .unwrap_or(&mut io::stdout()),
                )
            }),
            output_dir,
            &output_file_path,
            question_policy,
        )? {
            files
        } else {
            return Ok(());
        };

        // this is only printed once, so it doesn't result in much text. On the other hand,
        // having a final status message is important especially in an accessibility context
        // as screen readers may not read a commands exit code, making it hard to reason
        // about whether the command succeeded without such a message
        info!(
            accessible,
            "Successfully decompressed archive in {} ({} files).",
            nice_directory_display(output_dir),
            files.len()
        );

        return Ok(());
    }

    // Will be used in decoder chaining
    let reader = BufReader::with_capacity(BUFFER_CAPACITY, reader);
    let mut reader: Box<dyn Read> = Box::new(reader);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| -> crate::Result<Box<dyn Read>> {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
            Lz4 => Box::new(lzzzz::lz4f::ReadDecompressor::new(decoder)?),
            Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
            Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Tar | Zip => unreachable!(),
        };
        Ok(decoder)
    };

    let (first_extension, extensions) = split_first_compression_format(&formats);

    for format in extensions.iter().rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files_unpacked = match first_extension {
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            reader = chain_reader_decoder(&first_extension, reader)?;

            let writer = utils::create_or_ask_overwrite(&output_file_path, question_policy)?;
            if writer.is_none() {
                // Means that the user doesn't want to overwrite
                return Ok(());
            }
            let mut writer = writer.unwrap();

            let current_position_fn = Box::new({
                let output_file_path = output_file_path.clone();
                move || output_file_path.clone().metadata().expect("file exists").len()
            });
            let _progress = Progress::new_accessible_aware(total_input_size, true, Some(current_position_fn));

            io::copy(&mut reader, &mut writer)?;
            vec![output_file_path]
        }
        Tar => {
            if let ControlFlow::Continue(files) = smart_unpack(
                Box::new(move |output_dir| {
                    let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                    crate::archive::tar::unpack_archive(
                        reader,
                        output_dir,
                        progress
                            .as_mut()
                            .map(Progress::display_handle)
                            .unwrap_or(&mut io::stdout()),
                    )
                }),
                output_dir,
                &output_file_path,
                question_policy,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        Zip => {
            if formats.len() > 1 {
                warn_user_about_in_memory_zip_decompression();

                // give user the option to continue decompressing after warning is shown
                if !user_wants_to_continue(input_file_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            if let ControlFlow::Continue(files) = smart_unpack(
                Box::new(move |output_dir| {
                    let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                    crate::archive::zip::unpack_archive(
                        zip_archive,
                        output_dir,
                        progress
                            .as_mut()
                            .map(Progress::display_handle)
                            .unwrap_or(&mut io::stdout()),
                    )
                }),
                output_dir,
                &output_file_path,
                question_policy,
            )? {
                files
            } else {
                return Ok(());
            }
        }
    };

    // this is only printed once, so it doesn't result in much text. On the other hand,
    // having a final status message is important especially in an accessibility context
    // as screen readers may not read a commands exit code, making it hard to reason
    // about whether the command succeeded without such a message
    info!(
        accessible,
        "Successfully decompressed archive in {}.",
        nice_directory_display(output_dir)
    );
    info!(accessible, "Files unpacked: {}", files_unpacked.len());

    Ok(())
}

/// Unpacks an archive with some heuristics
/// - If the archive contains only one file, it will be extracted to the `output_dir`
/// - If the archive contains multiple files, it will be extracted to a subdirectory of the output_dir named after the archive (given by `output_file_path`)
/// Note: This functions assumes that `output_dir` exists
fn smart_unpack(
    unpack_fn: Box<dyn FnOnce(&Path) -> crate::Result<Vec<PathBuf>>>,
    output_dir: &Path,
    output_file_path: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<(), Vec<PathBuf>>> {
    assert!(output_dir.exists());
    let temp_dir = tempfile::tempdir_in(output_dir)?;
    let temp_dir_path = temp_dir.path();
    info!(
        accessible,
        "Created temporary directory {} to hold decompressed elements.",
        nice_directory_display(temp_dir_path)
    );

    // unpack the files
    let files = unpack_fn(temp_dir_path)?;

    let root_contains_only_one_element = fs::read_dir(&temp_dir_path)?.count() == 1;
    if root_contains_only_one_element {
        // Only one file in the root directory, so we can just move it to the output directory
        let file = fs::read_dir(&temp_dir_path)?.next().expect("item exists")?;
        let file_path = file.path();
        let file_name = file_path
            .file_name()
            .expect("Should be safe because paths in archives should not end with '..'");
        let correct_path = output_dir.join(file_name);
        // One case to handle tough is we need to check if a file with the same name already exists
        if !utils::clear_path(&correct_path, question_policy)? {
            return Ok(ControlFlow::Break(()));
        }
        fs::rename(&file_path, &correct_path)?;
        info!(
            accessible,
            "Successfully moved {} to {}.",
            nice_directory_display(&file_path),
            nice_directory_display(&correct_path)
        );
    } else {
        // Multiple files in the root directory, so:
        // Rename  the temporary directory to the archive name, which is output_file_path
        // One case to handle tough is we need to check if a file with the same name already exists
        if !utils::clear_path(output_file_path, question_policy)? {
            return Ok(ControlFlow::Break(()));
        }
        fs::rename(&temp_dir_path, &output_file_path)?;
        info!(
            accessible,
            "Successfully moved {} to {}.",
            nice_directory_display(temp_dir_path),
            nice_directory_display(output_file_path)
        );
    }
    Ok(ControlFlow::Continue(files))
}
