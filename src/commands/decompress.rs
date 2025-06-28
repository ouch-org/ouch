use std::{
    io::{self, BufReader, Read},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use fs_err as fs;

#[cfg(not(feature = "bzip3"))]
use crate::archive;
use crate::{
    commands::{warn_user_about_loading_sevenz_in_memory, warn_user_about_loading_zip_in_memory},
    extension::{
        split_first_compression_format,
        CompressionFormat::{self, *},
        Extension,
    },
    utils::{
        self,
        io::lock_and_flush_output_stdio,
        is_path_stdin,
        logger::{info, info_accessible},
        nice_directory_display, user_wants_to_continue,
    },
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};

trait ReadSeek: Read + io::Seek {}
impl<T: Read + io::Seek> ReadSeek for T {}

pub struct DecompressOptions<'a> {
    pub input_file_path: &'a Path,
    pub formats: Vec<Extension>,
    pub output_dir: &'a Path,
    pub output_file_path: PathBuf,
    pub is_output_dir_provided: bool,
    pub is_smart_unpack: bool,
    pub question_policy: QuestionPolicy,
    pub quiet: bool,
    pub password: Option<&'a [u8]>,
    pub remove: bool,
}

/// Decompress a file
///
/// File at input_file_path is opened for reading, example: "archive.tar.gz"
/// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
/// output_dir it's where the file will be decompressed to, this function assumes that the directory exists
/// output_file_path is only used when extracting single file formats, not archive formats like .tar or .zip
pub fn decompress_file(options: DecompressOptions) -> crate::Result<()> {
    assert!(options.output_dir.exists());
    let input_is_stdin = is_path_stdin(options.input_file_path);

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
    }] = options.formats.as_slice()
    {
        let mut vec = vec![];
        let reader: Box<dyn ReadSeek> = if input_is_stdin {
            warn_user_about_loading_zip_in_memory();
            io::copy(&mut io::stdin(), &mut vec)?;
            Box::new(io::Cursor::new(vec))
        } else {
            Box::new(fs::File::open(options.input_file_path)?)
        };
        let zip_archive = zip::ZipArchive::new(reader)?;
        let files_unpacked = if let ControlFlow::Continue(files) = execute_decompression(
            |output_dir| crate::archive::zip::unpack_archive(zip_archive, output_dir, options.password, options.quiet),
            options.output_dir,
            &options.output_file_path,
            options.question_policy,
            options.is_output_dir_provided,
            options.is_smart_unpack,
        )? {
            files
        } else {
            return Ok(());
        };

        // this is only printed once, so it doesn't result in much text. On the other hand,
        // having a final status message is important especially in an accessibility context
        // as screen readers may not read a commands exit code, making it hard to reason
        // about whether the command succeeded without such a message
        info_accessible(format!(
            "Successfully decompressed archive in {} ({} files)",
            nice_directory_display(options.output_dir),
            files_unpacked
        ));

        if !input_is_stdin && options.remove {
            fs::remove_file(options.input_file_path)?;
            info(format!(
                "Removed input file {}",
                nice_directory_display(options.input_file_path)
            ));
        }

        return Ok(());
    }

    // Will be used in decoder chaining
    let reader: Box<dyn Read> = if input_is_stdin {
        Box::new(io::stdin())
    } else {
        Box::new(fs::File::open(options.input_file_path)?)
    };
    let reader = BufReader::with_capacity(BUFFER_CAPACITY, reader);
    let mut reader: Box<dyn Read> = Box::new(reader);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| -> crate::Result<Box<dyn Read>> {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
            Bzip3 => {
                #[cfg(not(feature = "bzip3"))]
                return Err(archive::bzip3_stub::no_support());

                #[cfg(feature = "bzip3")]
                Box::new(bzip3::read::Bz3Decoder::new(decoder)?)
            }
            Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(decoder)),
            Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
            Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Brotli => Box::new(brotli::Decompressor::new(decoder, BUFFER_CAPACITY)),
            Tar | Zip | Rar | SevenZip => decoder,
        };
        Ok(decoder)
    };

    let (first_extension, extensions) = split_first_compression_format(&options.formats);

    for format in extensions.iter().rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files_unpacked = match first_extension {
        Gzip | Bzip | Bzip3 | Lz4 | Lzma | Snappy | Zstd | Brotli => {
            reader = chain_reader_decoder(&first_extension, reader)?;

            let mut writer = match utils::ask_to_create_file(
                &options.output_file_path,
                options.question_policy,
                QuestionAction::Decompression,
            )? {
                Some(file) => file,
                None => return Ok(()),
            };

            io::copy(&mut reader, &mut writer)?;

            1
        }
        Tar => {
            if let ControlFlow::Continue(files) = execute_decompression(
                |output_dir| crate::archive::tar::unpack_archive(reader, output_dir, options.quiet),
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
                options.is_smart_unpack,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        Zip => {
            if options.formats.len() > 1 {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

                warn_user_about_loading_zip_in_memory();
                if !user_wants_to_continue(
                    options.input_file_path,
                    options.question_policy,
                    QuestionAction::Decompression,
                )? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            if let ControlFlow::Continue(files) = execute_decompression(
                |output_dir| {
                    crate::archive::zip::unpack_archive(zip_archive, output_dir, options.password, options.quiet)
                },
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
                options.is_smart_unpack,
            )? {
                files
            } else {
                return Ok(());
            }
        }
        #[cfg(feature = "unrar")]
        Rar => {
            type UnpackResult = crate::Result<usize>;
            let unpack_fn: Box<dyn FnOnce(&Path) -> UnpackResult> = if options.formats.len() > 1 || input_is_stdin {
                let mut temp_file = tempfile::NamedTempFile::new()?;
                io::copy(&mut reader, &mut temp_file)?;
                Box::new(move |output_dir| {
                    crate::archive::rar::unpack_archive(temp_file.path(), output_dir, options.password, options.quiet)
                })
            } else {
                Box::new(|output_dir| {
                    crate::archive::rar::unpack_archive(
                        options.input_file_path,
                        output_dir,
                        options.password,
                        options.quiet,
                    )
                })
            };

            if let ControlFlow::Continue(files) = execute_decompression(
                unpack_fn,
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
                options.is_smart_unpack,
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
        SevenZip => {
            if options.formats.len() > 1 {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

                warn_user_about_loading_sevenz_in_memory();
                if !user_wants_to_continue(
                    options.input_file_path,
                    options.question_policy,
                    QuestionAction::Decompression,
                )? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;

            if let ControlFlow::Continue(files) = execute_decompression(
                |output_dir| {
                    crate::archive::sevenz::decompress_sevenz(
                        io::Cursor::new(vec),
                        output_dir,
                        options.password,
                        options.quiet,
                    )
                },
                options.output_dir,
                &options.output_file_path,
                options.question_policy,
                options.is_output_dir_provided,
                options.is_smart_unpack,
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
    info_accessible(format!(
        "Successfully decompressed archive in {}",
        nice_directory_display(options.output_dir)
    ));
    info_accessible(format!("Files unpacked: {}", files_unpacked));

    if !input_is_stdin && options.remove {
        fs::remove_file(options.input_file_path)?;
        info(format!(
            "Removed input file {}",
            nice_directory_display(options.input_file_path)
        ));
    }

    Ok(())
}

fn execute_decompression(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<usize>,
    output_dir: &Path,
    output_file_path: &Path,
    question_policy: QuestionPolicy,
    is_output_dir_provided: bool,
    is_smart_unpack: bool,
) -> crate::Result<ControlFlow<(), usize>> {
    if is_smart_unpack {
        return smart_unpack(unpack_fn, output_dir, output_file_path, question_policy);
    }

    let target_output_dir = if is_output_dir_provided {
        output_dir
    } else {
        output_file_path
    };

    unpack(unpack_fn, target_output_dir, question_policy)
}

/// Unpacks an archive creating the output directory, this function will create the output_dir
/// directory or replace it if it already exists. The `output_dir` needs to be empty
/// - If `output_dir` does not exist OR is a empty directory, it will unpack there
/// - If `output_dir` exist OR is a directory not empty, the user will be asked what to do
fn unpack(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<usize>,
    output_dir: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<(), usize>> {
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

    let files = unpack_fn(output_dir_cleaned)?;

    Ok(ControlFlow::Continue(files))
}

/// Unpacks an archive with some heuristics
/// - If the archive contains only one file, it will be extracted to the `output_dir`
/// - If the archive contains multiple files, it will be extracted to a subdirectory of the
///   output_dir named after the archive (given by `output_file_path`)
///
/// Note: This functions assumes that `output_dir` exists
fn smart_unpack(
    unpack_fn: impl FnOnce(&Path) -> crate::Result<usize>,
    output_dir: &Path,
    output_file_path: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<(), usize>> {
    assert!(output_dir.exists());
    let temp_dir = tempfile::Builder::new().prefix("tmp-ouch-").tempdir_in(output_dir)?;
    let temp_dir_path = temp_dir.path();

    info_accessible(format!(
        "Created temporary directory {} to hold decompressed elements",
        nice_directory_display(temp_dir_path)
    ));

    let files = unpack_fn(temp_dir_path)?;

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
    fs::rename(&previous_path, &new_path)?;
    info_accessible(format!(
        "Successfully moved \"{}\" to \"{}\"",
        nice_directory_display(&previous_path),
        nice_directory_display(&new_path),
    ));

    Ok(ControlFlow::Continue(files))
}
