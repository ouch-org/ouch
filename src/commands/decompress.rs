use std::{
    io::{self, BufReader, Read},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    commands::{warn_user_about_loading_sevenz_in_memory, warn_user_about_loading_zip_in_memory},
    extension::{
        split_first_compression_format,
        CompressionFormat::{self, *},
        Extension,
    },
    info, info_accessible,
    utils::{
        self,
        io::{lock_and_flush_output_stdio, ReadSeek},
        is_path_stdin, nice_directory_display, set_permission_mode, user_wants_to_continue,
    },
    QuestionAction, QuestionPolicy, Result, BUFFER_CAPACITY,
};

pub type Mode = u32;
pub type UnpackRes = crate::Result<Unpacked>;

pub struct Unpacked {
    pub files_unpacked: usize,
    pub read_only_directories: Vec<(PathBuf, Mode)>,
}

pub struct DecompressOptions<'a> {
    /// Example: "archive.tar.gz"
    pub input_file_path: &'a Path,
    /// Example: [Gz, Tar] (notice it's ordered in decompression order)
    pub formats: Vec<Extension>,
    pub output_dir: &'a Path,
    /// Used when extracting single file formats and not archive formats
    pub output_file_path: PathBuf,
    pub is_output_dir_provided: bool,
    pub question_policy: QuestionPolicy,
    pub password: Option<&'a [u8]>,
    pub remove: bool,
}

/// Decompress (or unpack) a compressed (or packed) file.
pub fn decompress_file(options: DecompressOptions) -> crate::Result<()> {
    assert!(options.output_dir.try_exists()?);

    let input_is_stdin = is_path_stdin(options.input_file_path);
    let (first_extension, extensions) = split_first_compression_format(&options.formats);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| -> crate::Result<Box<dyn Read>> {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
            Bzip3 => {
                #[cfg(not(feature = "bzip3"))]
                return Err(crate::archive::bzip3_stub::no_support());
                #[cfg(feature = "bzip3")]
                Box::new(bzip3::read::Bz3Decoder::new(decoder)?)
            }
            Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(decoder)),
            Lzma => Box::new(lzma_rust2::LzmaReader::new_mem_limit(decoder, u32::MAX, None)?),
            Xz => Box::new(lzma_rust2::XzReader::new(decoder, true)),
            Lzip => Box::new(lzma_rust2::LzipReader::new(decoder)?),
            Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Brotli => Box::new(brotli::Decompressor::new(decoder, BUFFER_CAPACITY)),
            Tar | Zip | Rar | SevenZip => unreachable!(),
        };
        Ok(decoder)
    };

    let create_decoder_up_to_first_extension = || -> Result<Box<dyn Read>> {
        let mut reader: Box<dyn Read> = if input_is_stdin {
            Box::new(io::stdin())
        } else {
            Box::new(BufReader::with_capacity(
                BUFFER_CAPACITY,
                fs::File::open(options.input_file_path)?,
            ))
        };

        for format in extensions.iter().rev() {
            reader = chain_reader_decoder(format, reader)?;
        }

        Ok(reader)
    };

    let Unpacked { files_unpacked, .. } = match first_extension {
        Gzip | Bzip | Bzip3 | Lz4 | Lzma | Xz | Lzip | Snappy | Zstd | Brotli => {
            let reader = create_decoder_up_to_first_extension()?;
            let mut reader = chain_reader_decoder(&first_extension, reader)?;

            let mut writer = match utils::ask_to_create_file(
                &options.output_file_path,
                options.question_policy,
                QuestionAction::Decompression,
            )? {
                Some(file) => file,
                None => return Ok(()),
            };

            io::copy(&mut reader, &mut writer)?;

            Unpacked {
                files_unpacked: 1,
                read_only_directories: Vec::new(),
            }
        }
        Tar => {
            if let ControlFlow::Continue(files) = execute_decompression(
                |output_dir| crate::archive::tar::unpack_archive(create_decoder_up_to_first_extension()?, output_dir),
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        Zip | SevenZip => {
            let unpack_fn = match first_extension {
                Zip => crate::archive::zip::unpack_archive,
                SevenZip => crate::archive::sevenz::decompress_sevenz,
                _ => unreachable!(),
            };

            let should_load_everything_into_memory = input_is_stdin || !extensions.is_empty();

            // due to `io::Seek` being required by `Zip` and `SevenZip`, we might have to
            // copy all contents into a Vec to pass an `io::Cursor` (impls Seek)
            let reader: Box<dyn ReadSeek> = if should_load_everything_into_memory {
                let memory_warning_fn = match first_extension {
                    Zip => warn_user_about_loading_zip_in_memory,
                    SevenZip => warn_user_about_loading_sevenz_in_memory,
                    _ => unreachable!(),
                };

                // Make thread own locks to keep output messages adjacent
                let locks = lock_and_flush_output_stdio();
                memory_warning_fn();
                if !user_wants_to_continue(
                    options.input_file_path,
                    options.question_policy,
                    QuestionAction::Decompression,
                )? {
                    return Ok(());
                }
                drop(locks);

                let mut vec = vec![];
                io::copy(&mut create_decoder_up_to_first_extension()?, &mut vec)?;
                Box::new(io::Cursor::new(vec))
            } else {
                Box::new(BufReader::with_capacity(
                    BUFFER_CAPACITY,
                    fs::File::open(options.input_file_path)?,
                ))
            };

            if let ControlFlow::Continue(files) = execute_decompression(
                |output_dir| unpack_fn(reader, output_dir, options.password),
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        #[cfg(feature = "unrar")]
        Rar => {
            let unpack_fn: Box<dyn FnOnce(&Path) -> UnpackRes> = if options.formats.len() > 1 || input_is_stdin {
                let mut temp_file = tempfile::NamedTempFile::new()?;
                io::copy(&mut create_decoder_up_to_first_extension()?, &mut temp_file)?;
                Box::new(move |output_dir| {
                    crate::archive::rar::unpack_archive(temp_file.path(), output_dir, options.password)
                })
            } else {
                Box::new(|output_dir| {
                    crate::archive::rar::unpack_archive(options.input_file_path, output_dir, options.password)
                })
            };

            if let ControlFlow::Continue(files) = execute_decompression(
                unpack_fn,
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        #[cfg(not(feature = "unrar"))]
        Rar => {
            return Err(crate::archive::rar_stub::no_support());
        }
    };

    // this is only printed once, so it doesn't result in much text. On the other hand,
    // having a final status message is important especially in an accessibility context
    // as screen readers may not read a commands exit code, making it hard to reason
    // about whether the command succeeded without such a message
    info_accessible!(
        "Successfully decompressed archive in {}",
        nice_directory_display(options.output_dir)
    );
    info_accessible!("Files unpacked: {files_unpacked}");

    if !input_is_stdin && options.remove {
        fs::remove_file(options.input_file_path)?;
        info!("Removed input file {}", nice_directory_display(options.input_file_path));
    }

    Ok(())
}

fn execute_decompression(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<Unpacked>,
    output_dir: &Path,
    output_file_path: &Path,
    question_policy: QuestionPolicy,
    is_output_dir_provided: bool,
) -> crate::Result<ControlFlow<(), Unpacked>> {
    if is_output_dir_provided {
        unpack(unpack_fn, output_dir, question_policy)
    } else {
        smart_unpack(unpack_fn, output_dir, output_file_path, question_policy)
    }
}

/// Unpacks an archive creating the output directory, this function will create the output_dir
/// directory or replace it if it already exists. The `output_dir` needs to be empty
/// - If `output_dir` does not exist OR is a empty directory, it will unpack there
/// - If `output_dir` exist OR is a directory not empty, the user will be asked what to do
fn unpack(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<Unpacked>,
    output_dir: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<(), Unpacked>> {
    let is_valid_output_dir = !output_dir.exists() || (output_dir.is_dir() && output_dir.read_dir()?.next().is_none());

    let output_dir_cleaned = if is_valid_output_dir {
        output_dir.to_owned()
    } else {
        match utils::resolve_path_conflict(output_dir, question_policy, QuestionAction::Decompression)? {
            Some(path) => path,
            None => return Ok(ControlFlow::Break(())),
        }
    };

    if !output_dir_cleaned.exists() {
        fs::create_dir(&output_dir_cleaned)?;
    }

    let files = unpack_fn(&output_dir_cleaned)?;

    Ok(ControlFlow::Continue(files))
}

/// Unpacks an archive with some heuristics
/// - If the archive contains only one file, it will be extracted to the `output_dir`
/// - If the archive contains multiple files, it will be extracted to a subdirectory of the
///   output_dir named after the archive (given by `output_file_path`)
///
/// Note: This functions assumes that `output_dir` exists
fn smart_unpack(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<Unpacked>,
    output_dir: &Path,
    output_file_path: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<(), Unpacked>> {
    assert!(output_dir.exists());
    let temp_dir = tempfile::Builder::new().prefix("tmp-ouch-").tempdir_in(output_dir)?;
    let temp_dir_path = temp_dir.path();

    info_accessible!(
        "Created temporary directory {} to hold decompressed elements",
        nice_directory_display(temp_dir_path)
    );

    let Unpacked {
        files_unpacked,
        read_only_directories,
    } = unpack_fn(temp_dir_path)?;

    let root_contains_only_one_element = fs::read_dir(temp_dir_path)?.take(2).count() == 1;

    let (previous_path, mut new_path) = if root_contains_only_one_element {
        // Only one file in the root directory, so we can just move it to the output directory
        let file = fs::read_dir(temp_dir_path)?.next().expect("item exists")?;
        let file_path = file.path();
        let file_name = file_path
            .file_name()
            .expect("Should be safe because paths in archives should not end with '..'");
        let correct_path = output_dir.join(file_name);

        (file_path, correct_path)
    } else {
        (temp_dir_path.to_owned(), output_file_path.to_owned())
    };

    // Before moving, need to check if a file with the same name already exists
    // If it does, need to ask the user what to do
    new_path = match utils::resolve_path_conflict(&new_path, question_policy, QuestionAction::Decompression)? {
        Some(path) => path,
        None => return Ok(ControlFlow::Break(())),
    };

    // Rename the temporary directory to the archive name, which is output_file_path
    if fs::rename(&previous_path, &new_path).is_err() {
        utils::rename_recursively(&previous_path, &new_path)?;
    };

    if cfg!(unix) {
        for (path, mode) in &read_only_directories {
            let components = path.components();
            let mut path = new_path.clone();
            for component in components {
                path.push(component);
            }
            set_permission_mode(&path, *mode)?;
        }
    }

    info_accessible!(
        "Successfully moved \"{}\" to \"{}\"",
        nice_directory_display(&previous_path),
        nice_directory_display(&new_path),
    );

    Ok(ControlFlow::Continue(Unpacked {
        files_unpacked,
        read_only_directories,
    }))
}
