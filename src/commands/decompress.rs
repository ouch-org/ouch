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
        is_path_stdin, nice_directory_display, user_wants_to_continue,
    },
    QuestionAction, QuestionPolicy, Result, BUFFER_CAPACITY,
};

pub struct DecompressOptions<'a> {
    /// Example: "archive.tar.gz"
    pub input_file_path: &'a Path,
    /// Example: [Gz, Tar] (notice it's ordered in decompression order)
    pub formats: Vec<Extension>,
    pub output_dir: &'a Path,
    /// Used when extracting single file formats and not archive formats
    pub output_file_path: PathBuf,
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

    let control_flow = match first_extension {
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
            ControlFlow::Continue(1)
        }
        Tar => unpack_archive(
            |output_dir| crate::archive::tar::unpack_archive(create_decoder_up_to_first_extension()?, output_dir),
            options.output_dir,
            options.question_policy,
        )?,
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

            unpack_archive(
                |output_dir| unpack_fn(reader, output_dir, options.password),
                options.output_dir,
                options.question_policy,
            )?
        }
        #[cfg(feature = "unrar")]
        Rar => {
            let unpack_fn: Box<dyn FnOnce(&Path) -> Result<usize>> = if options.formats.len() > 1 || input_is_stdin {
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

            unpack_archive(unpack_fn, options.output_dir, options.question_policy)?
        }
        #[cfg(not(feature = "unrar"))]
        Rar => {
            return Err(crate::archive::rar_stub::no_support());
        }
    };

    let ControlFlow::Continue(files_unpacked) = control_flow else {
        return Ok(());
    };

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

/// Unpacks an archive creating the output directory, this function will create the output_dir
/// directory or replace it if it already exists. The `output_dir` needs to be empty
/// - If `output_dir` does not exist OR is a empty directory, it will unpack there
/// - If `output_dir` exist OR is a directory not empty, the user will be asked what to do
fn unpack_archive(
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

    let files = unpack_fn(&output_dir_cleaned)?;

    Ok(ControlFlow::Continue(files))
}
