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
    list::{self, ListOptions},
    utils::{
        self, concatenate_os_str_list, dir_is_empty, nice_directory_display, to_utf, try_infer_extension,
        user_wants_to_continue_decompressing,
    },
    warning, Opts, QuestionPolicy, Subcommand,
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
        Subcommand::Compress { files, output: output_path } => {
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
                let pos = extensions_start_position;
                let empty_range = pos..pos;
                let mut suggested_output_path = output_path.clone();
                suggested_output_path.replace_range(empty_range, ".tar");

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
                        "Partial compression detected. Compressing {} into {}",
                        to_utf(files[0].as_path().file_name().unwrap()),
                        to_utf(&output_path)
                    );
                    formats = new_formats;
                }
            }
            let compress_result = compress_files(files, formats, output_file);

            // If any error occurred, delete incomplete file
            if compress_result.is_err() {
                // Print an extra alert message pointing out that we left a possibly
                // CORRUPTED FILE at `output_path`
                if let Err(err) = fs::remove_file(&output_path) {
                    eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                    eprintln!("  Please manually delete '{}'.", to_utf(&output_path));
                    eprintln!("  Compression failed and we could not delete '{}'.", to_utf(&output_path),);
                    eprintln!("  Error:{reset} {}{red}.{reset}\n", err, reset = *colors::RESET, red = *colors::RED);
                }
            } else {
                info!("Successfully compressed '{}'.", to_utf(output_path));
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

            // From Option<PathBuf> to Option<&Path>
            let output_dir = output_dir.as_ref().map(|path| path.as_ref());

            for ((input_path, formats), file_name) in files.iter().zip(formats).zip(output_paths) {
                decompress_file(input_path, formats, output_dir, file_name, question_policy)?;
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
                list_archive_contents(archive_path, formats, list_options)?;
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
fn compress_files(files: Vec<PathBuf>, formats: Vec<Extension>, output_file: fs::File) -> crate::Result<()> {
    let file_writer = BufWriter::with_capacity(BUFFER_CAPACITY, output_file);

    let mut writer: Box<dyn Write> = Box::new(file_writer);

    // Grab previous encoder and wrap it inside of a new one
    let chain_writer_encoder = |format: &CompressionFormat, encoder: Box<dyn Write>| -> crate::Result<Box<dyn Write>> {
        let encoder: Box<dyn Write> = match format {
            Gzip => Box::new(flate2::write::GzEncoder::new(encoder, Default::default())),
            Bzip => Box::new(bzip2::write::BzEncoder::new(encoder, Default::default())),
            Lz4 => Box::new(lzzzz::lz4f::WriteCompressor::new(encoder, Default::default())?),
            Lzma => Box::new(xz2::write::XzEncoder::new(encoder, 6)),
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
        Gzip | Bzip | Lz4 | Lzma | Zstd => {
            writer = chain_writer_encoder(&formats[0].compression_formats[0], writer)?;
            let mut reader = fs::File::open(&files[0]).unwrap();
            io::copy(&mut reader, &mut writer)?;
        }
        Tar => {
            let mut writer = archive::tar::build_archive_from_paths(&files, writer)?;
            writer.flush()?;
        }
        Zip => {
            eprintln!("{yellow}Warning:{reset}", yellow = *colors::YELLOW, reset = *colors::RESET);
            eprintln!("\tCompressing .zip entirely in memory.");
            eprintln!("\tIf the file is too big, your PC might freeze!");
            eprintln!(
                "\tThis is a limitation for formats like '{}'.",
                formats.iter().map(|format| format.to_string()).collect::<String>()
            );
            eprintln!("\tThe design of .zip makes it impossible to compress via stream.");

            let mut vec_buffer = io::Cursor::new(vec![]);
            archive::zip::build_archive_from_paths(&files, &mut vec_buffer)?;
            let vec_buffer = vec_buffer.into_inner();
            io::copy(&mut vec_buffer.as_slice(), &mut writer)?;
        }
    }

    Ok(())
}

enum OutputKind {
    UserSelected(PathBuf),
    AutoSelected(PathBuf),
}
impl OutputKind {
    fn as_path(&self) -> &Path {
        match self {
            Self::UserSelected(path) => path,
            Self::AutoSelected(path) => path,
        }
    }
}

// Decompress a file
//
// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
// output_dir it's where the file will be decompressed to
// file_name is only used when extracting single file formats, no archive formats like .tar or .zip
fn decompress_file(
    input_file_path: &Path,
    formats: Vec<Extension>,
    output_dir: Option<&Path>,
    file_name: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<()> {
    let reader = fs::File::open(&input_file_path)?;

    // Output path used by single file formats
    let single_file_format_output_path =
        if let Some(output_dir) = output_dir { output_dir.join(file_name) } else { file_name.to_path_buf() };

    // Output folder used by archive file formats (zip and tar)
    let archive_output_dir = output_dir
        .map(|dir| OutputKind::UserSelected(dir.to_path_buf()))
        .unwrap_or_else(|| OutputKind::AutoSelected(single_file_format_output_path.clone()));

    // Zip archives are special, because they require io::Seek, so it requires it's logic separated
    // from decoder chaining.
    //
    // This is the only case where we can read and unpack it directly, without having to do
    // in-memory decompression/copying first.
    //
    // Any other Zip decompression done can take up the whole RAM and freeze ouch.
    if formats.len() == 1 && *formats[0].compression_formats == [Zip] {
        let zip_unpack = move |output_dir: PathBuf| -> crate::Result<Vec<PathBuf>> {
            let zip_archive = zip::ZipArchive::new(reader)?;
            crate::archive::zip::unpack_archive(zip_archive, &output_dir, question_policy)
        };

        if let ControlFlow::Continue((path, _)) =
            extract_archive_smart(Box::new(zip_unpack), question_policy, &archive_output_dir)?
        {
            info!("Successfully decompressed archive in {}.", nice_directory_display(path));
        }
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
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Tar | Zip => unreachable!(),
        };
        Ok(decoder)
    };

    for format in formats.iter().flat_map(Extension::iter).skip(1).collect::<Vec<_>>().iter().rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files_unpacked;
    let final_output_path;

    match formats[0].compression_formats[0] {
        Gzip | Bzip | Lz4 | Lzma | Zstd => {
            if let ControlFlow::Break(_) =
                utils::create_dir_or_ask_overwrite(&single_file_format_output_path, question_policy)?
            {
                return Ok(());
            };

            reader = chain_reader_decoder(&formats[0].compression_formats[0], reader)?;

            let writer = utils::create_file_or_ask_overwrite(&single_file_format_output_path, question_policy)?;
            if writer.is_none() {
                // Means that the user doesn't want to overwrite
                return Ok(());
            }
            let mut writer = writer.unwrap();

            io::copy(&mut reader, &mut writer)?;
            files_unpacked = vec![single_file_format_output_path.clone()];
            final_output_path = single_file_format_output_path;
        }
        Tar => {
            let tar_unpack =
                move |output_dir: PathBuf| crate::archive::tar::unpack_archive(reader, &output_dir, question_policy);
            match extract_archive_smart(Box::new(tar_unpack), question_policy, &archive_output_dir)? {
                ControlFlow::Continue((path, files)) => {
                    files_unpacked = files;
                    final_output_path = path;
                }
                ControlFlow::Break(_) => return Ok(()),
            }
        }
        Zip => {
            eprintln!("Compressing first into .zip.");
            eprintln!("Warning: .zip archives with extra extensions have a downside.");
            eprintln!(
                "The only way is loading everything into the RAM while compressing, and then write everything down."
            );
            eprintln!("this means that by compressing .zip with extra compression formats, you can run out of RAM if the file is too large!");

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            let zip_unpack = move |output_dir: PathBuf| {
                crate::archive::zip::unpack_archive(zip_archive, &output_dir, question_policy)
            };
            match extract_archive_smart(Box::new(zip_unpack), question_policy, &archive_output_dir)? {
                ControlFlow::Continue((path, files)) => {
                    files_unpacked = files;
                    final_output_path = path;
                }
                ControlFlow::Break(_) => {
                    return Ok(());
                }
            }
        }
    }

    info!("Successfully decompressed archive in {}.", nice_directory_display(final_output_path));
    info!("Files unpacked: {}", files_unpacked.len());

    Ok(())
}

/// Extract an archive using an unpack function
/// 1- If the archive has only one element, we extract that to the current directory
/// 2- If the archive has many elements at its root, we create a directory that contains all of them
/// Note: If the user has specified an output directory (with --dir), the output elements are
/// always extracted inside it
/// Returns - A ControlFlow::Continue with the output path and the elements unpacked
///         - A ControlFlow::Break if an overwrite was needed and the user declined
fn extract_archive_smart(
    unpack_fn: Box<dyn FnOnce(PathBuf) -> crate::Result<Vec<PathBuf>>>,
    question_policy: QuestionPolicy,
    output_path: &OutputKind,
) -> Result<ControlFlow<(), (PathBuf, Vec<PathBuf>)>, crate::Error> {
    // In both cases we start by creating a temporary directory to hold the elements
    let output_path_parent = output_path.as_path().parent().unwrap_or_else(|| Path::new("."));

    let temp_output_dir_guard = tempfile::tempdir_in(output_path_parent)?;
    let temp_output_path = temp_output_dir_guard.path();
    utils::create_dir_if_non_existent(temp_output_path)?;
    info!("Created temporary directory {} to hold decompressed elements.", nice_directory_display(temp_output_path));

    // unpack the elements
    let elements = unpack_fn(temp_output_path.to_path_buf())?;

    let root_contains_only_one_element = fs::read_dir(&temp_output_path)?.count() == 1;
    if root_contains_only_one_element {
        // first case: only one element in the archive, we extract it to the current directory
        let entry = fs::read_dir(&temp_output_path)?.next().unwrap().unwrap();
        let entry_path = entry.path();
        let entry_name = entry_path.file_name().unwrap().to_str().unwrap();
        let element_final_path = if let OutputKind::UserSelected(path) = output_path {
            // Even if this is the case of only one element, if the user did specify a directory it
            // would be surprising if he doesn't find the extracted files in it
            // So that's what we do here
            path.join(entry_name)
        } else {
            output_path_parent.join(entry_name)
        };

        // This is the path were the entry will be moved to
        if element_final_path.is_dir() {
            // If it is a directory and it already exists, we ask the user if he wants to overwrite
            if let ControlFlow::Break(_) = utils::create_dir_or_ask_overwrite(&element_final_path, question_policy)? {
                return Ok(ControlFlow::Break(()));
            };
        } else {
            // If it is a file and it already exists, we ask the user if he wants to overwrite
            if !utils::clear_path(&element_final_path, question_policy)? {
                // User doesn't want to overwrite
                return Ok(ControlFlow::Break(()));
            }
            // And we also create the directory where it will be moved to if doesn't exist
            utils::create_dir_if_non_existent(element_final_path.parent().unwrap())?;
        }

        std::fs::rename(&entry_path, element_final_path.clone())?;
        info!(
            "Successfully moved {} to {}.",
            nice_directory_display(&entry_path),
            nice_directory_display(&element_final_path)
        );
        Ok(ControlFlow::Continue((element_final_path, elements)))
    } else {
        // second case: many element in the archive, we extract them to a directory
        let output_path = output_path.as_path();
        if let ControlFlow::Break(_) = utils::create_dir_or_ask_overwrite(output_path, question_policy)? {
            return Ok(ControlFlow::Break(()));
        };

        std::fs::rename(temp_output_path, output_path)?;
        info!(
            "Successfully moved {} to {}.",
            nice_directory_display(&temp_output_path),
            nice_directory_display(&output_path)
        );
        Ok(ControlFlow::Continue((output_path.to_path_buf(), elements)))
    }
}

// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
fn list_archive_contents(
    archive_path: &Path,
    formats: Vec<CompressionFormat>,
    list_options: ListOptions,
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
        let files = crate::archive::zip::list_archive(zip_archive)?;
        list::list_files(archive_path, files, list_options);
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
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Tar | Zip => unreachable!(),
        };
        Ok(decoder)
    };

    for format in formats.iter().skip(1).rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    let files = match formats[0] {
        Tar => crate::archive::tar::list_archive(reader)?,
        Zip => {
            eprintln!("Listing files from zip archive.");
            eprintln!("Warning: .zip archives with extra extensions have a downside.");
            eprintln!("The only way is loading everything into the RAM while compressing, and then reading the archive contents.");
            eprintln!("this means that by compressing .zip with extra compression formats, you can run out of RAM if the file is too large!");

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            crate::archive::zip::list_archive(zip_archive)?
        }
        Gzip | Bzip | Lz4 | Lzma | Zstd => {
            panic!("Not an archive! This should never happen, if it does, something is wrong with `CompressionFormat::is_archive()`. Please report this error!");
        }
    };
    list::list_files(archive_path, files, list_options);
    Ok(())
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
                info!("Detected file: `{}` extension as `{}`", path.display(), detected_format);
                if user_wants_to_continue_decompressing(path, question_policy)? {
                    format.push(detected_format);
                } else {
                    return Ok(ControlFlow::Break(()));
                }
            }
        } else if let Some(detected_format) = try_infer_extension(path) {
            // File ending with extension
            // Try to detect the extension and warn the user if it differs from the written one
            let outer_ext = format.iter().next().unwrap();
            if outer_ext != &detected_format {
                warning!(
                    "The file extension: `{}` differ from the detected extension: `{}`",
                    outer_ext,
                    detected_format
                );
                if !user_wants_to_continue_decompressing(path, question_policy)? {
                    return Ok(ControlFlow::Break(()));
                }
            }
        } else {
            // NOTE: If this actually produces no false positives, we can upgrade it in the future
            // to a warning and ask the user if he wants to continue decompressing.
            info!("Could not detect the extension of `{}`", path.display());
        }
    }
    Ok(ControlFlow::Continue(()))
}
