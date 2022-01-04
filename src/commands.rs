//! Core of the crate, where the `compress_files` and `decompress_file` functions are implemented
//!
//! Also, where correctly call functions based on the detected `Command`.

use std::{
    io::{self, BufReader, BufWriter, Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use fs_err as fs;
use utils::colors;

use crate::{
    archive,
    error::FinalError,
    extension::{
        self,
        CompressionFormat::{self, *},
        Extension,
    },
    info,
    list::{self, FileInArchive, ListOptions},
    progress::Progress,
    utils::{
        self, concatenate_os_str_list, dir_is_empty, nice_directory_display, to_utf, try_infer_extension,
        user_wants_to_continue,
    },
    warning, Opts, QuestionAction, QuestionPolicy, Subcommand,
};

// Used in BufReader and BufWriter to perform less syscalls
const BUFFER_CAPACITY: usize = 1024 * 64;

fn represents_several_files(files: &[PathBuf]) -> bool {
    let is_non_empty_dir = |path: &PathBuf| {
        let is_non_empty = || !dir_is_empty(path);

        path.is_dir().then(is_non_empty).unwrap_or_default()
    };

    files.iter().any(is_non_empty_dir) || files.len() > 1
}

/// Entrypoint of ouch, receives cli options and matches Subcommand to decide what to do
pub fn run(args: Opts, question_policy: QuestionPolicy) -> crate::Result<()> {
    match args.cmd {
        Subcommand::Compress { mut files, output: output_path } => {
            // If the output_path file exists and is the same as some of the input files, warn the user and skip those inputs (in order to avoid compression recursion)
            if output_path.exists() {
                clean_input_files_if_needed(&mut files, &fs::canonicalize(&output_path)?);
            }
            // After cleaning, if there are no input files left, exit
            if files.is_empty() {
                return Err(FinalError::with_title("No files to compress").into());
            }

            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let mut formats = extension::extensions_from_path(&output_path);

            if formats.is_empty() {
                let error = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail("You shall supply the compression format")
                    .hint("Try adding supported extensions (see --help):")
                    .hint(format!("  ouch compress <FILES>... {}.tar.gz", to_utf(&output_path)))
                    .hint(format!("  ouch compress <FILES>... {}.zip", to_utf(&output_path)))
                    .hint("")
                    .hint("Alternatively, you can overwrite this option by using the '--format' flag:")
                    .hint(format!("  ouch compress <FILES>... {} --format tar.gz", to_utf(&output_path)));

                return Err(error.into());
            }

            if !formats.get(0).map(Extension::is_archive).unwrap_or(false) && represents_several_files(&files) {
                // This piece of code creates a suggestion for compressing multiple files
                // It says:
                // Change from file.bz.xz
                // To          file.tar.bz.xz
                let extensions_text: String = formats.iter().map(|format| format.to_string()).collect();

                let output_path = to_utf(output_path);

                // Breaks if Lzma is .lz or .lzma and not .xz
                // Or if Bzip is .bz2 and not .bz
                let extensions_start_position = output_path.rfind(&extensions_text).unwrap();
                let pos = extensions_start_position - 1;
                let mut suggested_output_path = output_path.clone();
                suggested_output_path.insert_str(pos, ".tar");

                let error = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail("You are trying to compress multiple files.")
                    .detail(format!("The compression format '{}' cannot receive multiple files.", &formats[0]))
                    .detail("The only supported formats that archive files into an archive are .tar and .zip.")
                    .hint(format!("Try inserting '.tar' or '.zip' before '{}'.", &formats[0]))
                    .hint(format!("From: {}", output_path))
                    .hint(format!("To:   {}", suggested_output_path));

                return Err(error.into());
            }

            if let Some(format) = formats.iter().skip(1).find(|format| format.is_archive()) {
                let error = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail(format!("Found the format '{}' in an incorrect position.", format))
                    .detail(format!("'{}' can only be used at the start of the file extension.", format))
                    .hint(format!("If you wish to compress multiple files, start the extension with '{}'.", format))
                    .hint(format!("Otherwise, remove the last '{}' from '{}'.", format, to_utf(&output_path)));

                return Err(error.into());
            }

            if output_path.exists() && !utils::user_wants_to_overwrite(&output_path, question_policy)? {
                // User does not want to overwrite this file, skip and return without any errors
                return Ok(());
            }

            let output_file = fs::File::create(&output_path)?;

            if !represents_several_files(&files) {
                // It's possible the file is already partially compressed so we don't want to compress it again
                // `ouch compress file.tar.gz file.tar.gz.xz` should produce `file.tar.gz.xz` and not `file.tar.gz.tar.gz.xz`
                let input_extensions = extension::extensions_from_path(&files[0]);

                // We calculate the formats that are left if we filter out a sublist at the start of what we have that's the same as the input formats
                let mut new_formats = Vec::with_capacity(formats.len());
                for (inp_ext, out_ext) in input_extensions.iter().zip(&formats) {
                    if inp_ext.compression_formats == out_ext.compression_formats {
                        new_formats.push(out_ext.clone());
                    } else if inp_ext
                        .compression_formats
                        .iter()
                        .zip(out_ext.compression_formats.iter())
                        .all(|(inp, out)| inp == out)
                    {
                        let new_ext = Extension::new(
                            &out_ext.compression_formats[..inp_ext.compression_formats.len()],
                            &out_ext.display_text,
                        );
                        new_formats.push(new_ext);
                        break;
                    }
                }
                // If the input is a sublist at the start of `formats` then remove the extensions
                // Note: If input_extensions is empty then it will make `formats` empty too, which we don't want
                if !input_extensions.is_empty() && new_formats != formats {
                    // Safety:
                    //   We checked above that input_extensions isn't empty, so files[0] has an extension.
                    //
                    //   Path::extension says: "if there is no file_name, then there is no extension".
                    //   Contrapositive statement: "if there is extension, then there is file_name".
                    info!(
                        accessible, // important information
                        "Partial compression detected. Compressing {} into {}",
                        to_utf(files[0].as_path().file_name().unwrap()),
                        to_utf(&output_path)
                    );
                    formats = new_formats;
                }
            }
            let compress_result = compress_files(files, formats, output_file, &output_path, question_policy);

            if let Ok(true) = compress_result {
                // this is only printed once, so it doesn't result in much text. On the other hand,
                // having a final status message is important especially in an accessibility context
                // as screen readers may not read a commands exit code, making it hard to reason
                // about whether the command succeeded without such a message
                info!(accessible, "Successfully compressed '{}'.", to_utf(output_path));
            } else {
                // If Ok(false) or Err() occurred, delete incomplete file
                // Print an extra alert message pointing out that we left a possibly
                // CORRUPTED FILE at `output_path`
                if let Err(err) = fs::remove_file(&output_path) {
                    eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                    eprintln!("  Please manually delete '{}'.", to_utf(&output_path));
                    eprintln!("  Compression failed and we could not delete '{}'.", to_utf(&output_path),);
                    eprintln!("  Error:{reset} {}{red}.{reset}\n", err, reset = *colors::RESET, red = *colors::RED);
                }
            }

            compress_result?;
        }
        Subcommand::Decompress { files, output_dir } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            for path in files.iter() {
                let (file_output_path, file_formats) = extension::separate_known_extensions_from_name(path);
                output_paths.push(file_output_path);
                formats.push(file_formats);
            }

            if let ControlFlow::Break(_) = check_mime_type(&files, &mut formats, question_policy)? {
                return Ok(());
            }

            let files_missing_format: Vec<PathBuf> = files
                .iter()
                .zip(&formats)
                .filter(|(_, formats)| formats.is_empty())
                .map(|(input_path, _)| PathBuf::from(input_path))
                .collect();

            if !files_missing_format.is_empty() {
                let error = FinalError::with_title("Cannot decompress files without extensions")
                    .detail(format!(
                        "Files without supported extensions: {}",
                        concatenate_os_str_list(&files_missing_format)
                    ))
                    .detail("Decompression formats are detected automatically by the file extension")
                    .hint("Provide a file with a supported extension:")
                    .hint("  ouch decompress example.tar.gz")
                    .hint("")
                    .hint("Or overwrite this option with the '--format' flag:")
                    .hint(format!("  ouch decompress {} --format tar.gz", to_utf(&files_missing_format[0])));

                return Err(error.into());
            }

            // The directory that will contain the output files
            // We default to the current directory if the user didn't specify an output directory with --dir
            let output_dir = if let Some(dir) = output_dir {
                if !utils::clear_path(&dir, question_policy)? {
                    // User doesn't want to overwrite
                    return Ok(());
                }
                utils::create_dir_if_non_existent(&dir)?;
                dir
            } else {
                PathBuf::from(".")
            };

            for ((input_path, formats), file_name) in files.iter().zip(formats).zip(output_paths) {
                let output_file_path = output_dir.join(file_name); // Path used by single file format archives
                decompress_file(input_path, formats, &output_dir, output_file_path, question_policy)?;
            }
        }
        Subcommand::List { archives: files, tree } => {
            let mut formats = vec![];

            for path in files.iter() {
                let (_, file_formats) = extension::separate_known_extensions_from_name(path);
                formats.push(file_formats);
            }

            if let ControlFlow::Break(_) = check_mime_type(&files, &mut formats, question_policy)? {
                return Ok(());
            }

            let not_archives: Vec<PathBuf> = files
                .iter()
                .zip(&formats)
                .filter(|(_, formats)| !formats.get(0).map(Extension::is_archive).unwrap_or(false))
                .map(|(path, _)| path.clone())
                .collect();

            if !not_archives.is_empty() {
                let error = FinalError::with_title("Cannot list archive contents")
                    .detail("Only archives can have their contents listed")
                    .detail(format!("Files are not archives: {}", concatenate_os_str_list(&not_archives)));

                return Err(error.into());
            }

            let list_options = ListOptions { tree };

            for (i, (archive_path, formats)) in files.iter().zip(formats).enumerate() {
                if i > 0 {
                    println!();
                }
                let formats = formats.iter().flat_map(Extension::iter).map(Clone::clone).collect();
                list_archive_contents(archive_path, formats, list_options, question_policy)?;
            }
        }
    }
    Ok(())
}

// Compress files into an `output_file`
//
// files are the list of paths to be compressed: ["dir/file1.txt", "dir/file2.txt"]
// formats contains each format necessary for compression, example: [Tar, Gz] (in compression order)
// output_file is the resulting compressed file name, example: "compressed.tar.gz"
fn compress_files(
    files: Vec<PathBuf>,
    formats: Vec<Extension>,
    output_file: fs::File,
    output_dir: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<bool> {
    // The next lines are for displaying the progress bar
    // If the input files contain a directory, then the total size will be underestimated
    let (total_input_size, precise) = files
        .iter()
        .map(|f| (f.metadata().expect("file exists").len(), f.is_file()))
        .fold((0, true), |(total_size, and_precise), (size, precise)| (total_size + size, and_precise & precise));
    //NOTE: canonicalize is here to avoid a weird bug:
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

    for format in formats.iter().flat_map(Extension::iter).skip(1).collect::<Vec<_>>().iter().rev() {
        writer = chain_writer_encoder(format, writer)?;
    }

    match formats[0].compression_formats[0] {
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            let _progress = Progress::new_accessible_aware(
                total_input_size,
                precise,
                Some(Box::new(move || output_file_path.metadata().expect("file exists").len())),
            );

            writer = chain_writer_encoder(&formats[0].compression_formats[0], writer)?;
            let mut reader = fs::File::open(&files[0]).unwrap();
            io::copy(&mut reader, &mut writer)?;
        }
        Tar => {
            let mut progress = Progress::new_accessible_aware(
                total_input_size,
                precise,
                Some(Box::new(move || output_file_path.metadata().expect("file exists").len())),
            );

            archive::tar::build_archive_from_paths(
                &files,
                &mut writer,
                progress.as_mut().map(Progress::display_handle).unwrap_or(&mut io::stdout()),
            )?;
            writer.flush()?;
        }
        Zip => {
            if formats.len() > 1 {
                eprintln!("{orange}[WARNING]{reset}", orange = *colors::ORANGE, reset = *colors::RESET);
                eprintln!(
                    "\tThere is a limitation for .zip archives with extra extensions. (e.g. <file>.zip.gz)\
                    \n\tThe design of .zip makes it impossible to compress via stream, so it must be done entirely in memory.\
                    \n\tBy compressing .zip with extra compression formats, you can run out of RAM if the file is too large!"
                );

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
                progress.as_mut().map(Progress::display_handle).unwrap_or(&mut io::stdout()),
            )?;
            let vec_buffer = vec_buffer.into_inner();
            io::copy(&mut vec_buffer.as_slice(), &mut writer)?;
        }
    }

    Ok(true)
}

// Decompress a file
//
// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
// output_dir it's where the file will be decompressed to, this function assumes that the directory exists
// output_file_path is only used when extracting single file formats, not archive formats like .tar or .zip
fn decompress_file(
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
    if formats.len() == 1 && *formats[0].compression_formats == [Zip] {
        let zip_archive = zip::ZipArchive::new(reader)?;
        let files = if let ControlFlow::Continue(files) = smart_unpack(
            Box::new(move |output_dir| {
                let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                crate::archive::zip::unpack_archive(
                    zip_archive,
                    output_dir,
                    progress.as_mut().map(Progress::display_handle).unwrap_or(&mut io::stdout()),
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

    for format in formats.iter().flat_map(Extension::iter).skip(1).collect::<Vec<_>>().iter().rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files_unpacked;
    match formats[0].compression_formats[0] {
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            reader = chain_reader_decoder(&formats[0].compression_formats[0], reader)?;

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
            files_unpacked = vec![output_file_path];
        }
        Tar => {
            files_unpacked = if let ControlFlow::Continue(files) = smart_unpack(
                Box::new(move |output_dir| {
                    let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                    crate::archive::tar::unpack_archive(
                        reader,
                        output_dir,
                        progress.as_mut().map(Progress::display_handle).unwrap_or(&mut io::stdout()),
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
        }
        Zip => {
            if formats.len() > 1 {
                eprintln!("{orange}[WARNING]{reset}", orange = *colors::ORANGE, reset = *colors::RESET);
                eprintln!(
                    "\tThere is a limitation for .zip archives with extra extensions. (e.g. <file>.zip.gz)\
                    \n\tThe design of .zip makes it impossible to compress via stream, so it must be done entirely in memory.\
                    \n\tBy compressing .zip with extra compression formats, you can run out of RAM if the file is too large!"
                );

                // give user the option to continue decompressing after warning is shown
                if !user_wants_to_continue(input_file_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            files_unpacked = if let ControlFlow::Continue(files) = smart_unpack(
                Box::new(move |output_dir| {
                    let mut progress = Progress::new_accessible_aware(total_input_size, true, None);
                    crate::archive::zip::unpack_archive(
                        zip_archive,
                        output_dir,
                        progress.as_mut().map(Progress::display_handle).unwrap_or(&mut io::stdout()),
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
        }
    }

    // this is only printed once, so it doesn't result in much text. On the other hand,
    // having a final status message is important especially in an accessibility context
    // as screen readers may not read a commands exit code, making it hard to reason
    // about whether the command succeeded without such a message
    info!(accessible, "Successfully decompressed archive in {}.", nice_directory_display(output_dir));
    info!(accessible, "Files unpacked: {}", files_unpacked.len());

    Ok(())
}

// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
fn list_archive_contents(
    archive_path: &Path,
    formats: Vec<CompressionFormat>,
    list_options: ListOptions,
    question_policy: QuestionPolicy,
) -> crate::Result<()> {
    let reader = fs::File::open(&archive_path)?;

    // Zip archives are special, because they require io::Seek, so it requires it's logic separated
    // from decoder chaining.
    //
    // This is the only case where we can read and unpack it directly, without having to do
    // in-memory decompression/copying first.
    //
    // Any other Zip decompression done can take up the whole RAM and freeze ouch.
    if let [Zip] = *formats.as_slice() {
        let zip_archive = zip::ZipArchive::new(reader)?;
        let files = crate::archive::zip::list_archive(zip_archive);
        list::list_files(archive_path, files, list_options)?;

        return Ok(());
    }

    // Will be used in decoder chaining
    let reader = BufReader::with_capacity(BUFFER_CAPACITY, reader);
    let mut reader: Box<dyn Read + Send> = Box::new(reader);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder =
        |format: &CompressionFormat, decoder: Box<dyn Read + Send>| -> crate::Result<Box<dyn Read + Send>> {
            let decoder: Box<dyn Read + Send> = match format {
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

    for format in formats.iter().skip(1).rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files: Box<dyn Iterator<Item = crate::Result<FileInArchive>>> = match formats[0] {
        Tar => Box::new(crate::archive::tar::list_archive(tar::Archive::new(reader))),
        Zip => {

            if formats.len() > 1 {
                eprintln!("{orange}[WARNING]{reset}", orange = *colors::ORANGE, reset = *colors::RESET);
                eprintln!(
                    "\tThere is a limitation for .zip archives with extra extensions. (e.g. <file>.zip.gz)\
                    \n\tThe design of .zip makes it impossible to compress via stream, so it must be done entirely in memory.\
                    \n\tBy compressing .zip with extra compression formats, you can run out of RAM if the file is too large!"
                );

                // give user the option to continue decompressing after warning is shown
                if !user_wants_to_continue(archive_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            Box::new(crate::archive::zip::list_archive(zip_archive))
        }
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            panic!("Not an archive! This should never happen, if it does, something is wrong with `CompressionFormat::is_archive()`. Please report this error!");
        }
    };
    list::list_files(archive_path, files, list_options)?;
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
        let file_name =
            file_path.file_name().expect("Should be safe because paths in archives should not end with '..'");
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
            nice_directory_display(&temp_dir_path),
            nice_directory_display(&output_file_path)
        );
    }
    Ok(ControlFlow::Continue(files))
}

fn check_mime_type(
    files: &[PathBuf],
    formats: &mut Vec<Vec<Extension>>,
    question_policy: QuestionPolicy,
) -> crate::Result<ControlFlow<()>> {
    for (path, format) in files.iter().zip(formats.iter_mut()) {
        if format.is_empty() {
            // File with no extension
            // Try to detect it automatically and prompt the user about it
            if let Some(detected_format) = try_infer_extension(path) {
                // Infering the file extension can have unpredicted consequences (e.g. the user just
                // mistyped, ...) which we should always inform the user about.
                info!(accessible, "Detected file: `{}` extension as `{}`", path.display(), detected_format);
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
            if outer_ext != &detected_format {
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

fn clean_input_files_if_needed(files: &mut Vec<PathBuf>, output_path: &Path) {
    let mut idx = 0;
    while idx < files.len() {
        if files[idx] == output_path {
            warning!("The output file and the input file are the same: `{}`, skipping...", output_path.display());
            files.remove(idx);
        } else {
            idx += 1;
        }
    }
}
