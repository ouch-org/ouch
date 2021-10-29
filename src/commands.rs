//! Core of the crate, where the `compress_files` and `decompress_file` functions are implemented
//!
//! Also, where correctly call functions based on the detected `Command`.

use std::{
    fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use utils::colors;

use crate::{
    archive,
    cli::{Opts, Subcommand},
    error::FinalError,
    extension::{
        self,
        CompressionFormat::{self, *},
    },
    info,
    utils::nice_directory_display,
    utils::to_utf,
    utils::{self, dir_is_empty, QuestionPolicy},
    Error,
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

pub fn run(args: Opts, question_policy: QuestionPolicy) -> crate::Result<()> {
    match args.cmd {
        Subcommand::Compress { files, output: output_path } => {
            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let mut formats = extension::extensions_from_path(&output_path);

            if formats.is_empty() {
                let reason = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail("You shall supply the compression format via the extension.")
                    .hint("Try adding something like .tar.gz or .zip to the output file.")
                    .hint("")
                    .hint("Examples:")
                    .hint(format!("  ouch compress ... {}.tar.gz", to_utf(&output_path)))
                    .hint(format!("  ouch compress ... {}.zip", to_utf(&output_path)));

                return Err(Error::with_reason(reason));
            }

            if matches!(&formats[0], Bzip | Gzip | Lzma) && represents_several_files(&files) {
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

                let reason = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail("You are trying to compress multiple files.")
                    .detail(format!("The compression format '{}' cannot receive multiple files.", &formats[0]))
                    .detail("The only supported formats that archive files into an archive are .tar and .zip.")
                    .hint(format!("Try inserting '.tar' or '.zip' before '{}'.", &formats[0]))
                    .hint(format!("From: {}", output_path))
                    .hint(format!(" To : {}", suggested_output_path));

                return Err(Error::with_reason(reason));
            }

            if let Some(format) = formats.iter().skip(1).find(|format| matches!(format, Tar | Zip)) {
                let reason = FinalError::with_title(format!("Cannot compress to '{}'.", to_utf(&output_path)))
                    .detail(format!("Found the format '{}' in an incorrect position.", format))
                    .detail(format!("'{}' can only be used at the start of the file extension.", format))
                    .hint(format!("If you wish to compress multiple files, start the extension with '{}'.", format))
                    .hint(format!("Otherwise, remove the last '{}' from '{}'.", format, to_utf(&output_path)));

                return Err(Error::with_reason(reason));
            }

            if output_path.exists() && !utils::user_wants_to_overwrite(&output_path, question_policy)? {
                // User does not want to overwrite this file
                return Ok(());
            }

            let output_file = fs::File::create(&output_path)?;

            if !represents_several_files(&files) {
                // It's possible the file is already partially compressed so we don't want to compress it again
                // `ouch compress file.tar.gz file.tar.gz.xz` should produce `file.tar.gz.xz` and not `file.tar.gz.tar.gz.xz`
                let input_extensions = extension::extensions_from_path(&files[0]);

                // If the input is a sublist at the start of `formats` then remove the extensions
                // Note: If input_extensions is empty this counts as true
                if !input_extensions.is_empty()
                    && input_extensions.len() < formats.len()
                    && input_extensions.iter().zip(&formats).all(|(inp, out)| inp == out)
                {
                    // Safety:
                    //   We checked above that input_extensions isn't empty, so files[0] has a extension.
                    //
                    //   Path::extension says: "if there is no file_name, then there is no extension".
                    //   Using DeMorgan's law: "if there is    extension, then there is    file_name".
                    info!(
                        "Partial compression detected. Compressing {} into {}",
                        to_utf(files[0].as_path().file_name().unwrap()),
                        to_utf(&output_path)
                    );
                    let drain_iter = formats.drain(..input_extensions.len());
                    drop(drain_iter); // Remove the extensions from `formats`
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
        Subcommand::Decompress { files, output: output_folder } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            for path in files.iter() {
                let (file_output_path, file_formats) = extension::separate_known_extensions_from_name(path);
                output_paths.push(file_output_path);
                formats.push(file_formats);
            }

            let files_missing_format: Vec<PathBuf> = files
                .iter()
                .zip(&formats)
                .filter(|(_, formats)| formats.is_empty())
                .map(|(input_path, _)| PathBuf::from(input_path))
                .collect();

            // Error
            if !files_missing_format.is_empty() {
                eprintln!("Some file you asked ouch to decompress lacks a supported extension.");
                eprintln!("Could not decompress {}.", to_utf(&files_missing_format[0]));
                todo!(
                    "Dev note: add this error variant and pass the Vec to it, all the files \
                     lacking extension shall be shown: {:#?}.",
                    files_missing_format
                );
            }

            // From Option<PathBuf> to Option<&Path>
            let output_folder = output_folder.as_ref().map(|path| path.as_ref());

            for ((input_path, formats), file_name) in files.iter().zip(formats).zip(output_paths) {
                decompress_file(input_path, formats, output_folder, file_name, question_policy)?;
            }
        }
    }
    Ok(())
}

fn compress_files(files: Vec<PathBuf>, formats: Vec<CompressionFormat>, output_file: fs::File) -> crate::Result<()> {
    let file_writer = BufWriter::with_capacity(BUFFER_CAPACITY, output_file);

    if let [Tar | Tgz | Zip] = *formats.as_slice() {
        match formats[0] {
            Tar => {
                let mut bufwriter = archive::tar::build_archive_from_paths(&files, file_writer)?;
                bufwriter.flush()?;
            }
            Tgz => {
                // Wrap it into an gz_decoder, and pass to the tar archive builder
                let gz_decoder = flate2::write::GzEncoder::new(file_writer, Default::default());
                let mut bufwriter = archive::tar::build_archive_from_paths(&files, gz_decoder)?;
                bufwriter.flush()?;
            }
            Zip => {
                let mut bufwriter = archive::zip::build_archive_from_paths(&files, file_writer)?;
                bufwriter.flush()?;
            }
            _ => unreachable!(),
        };
    } else {
        let mut writer: Box<dyn Write> = Box::new(file_writer);

        // Grab previous encoder and wrap it inside of a new one
        let chain_writer_encoder = |format: &CompressionFormat, encoder: Box<dyn Write>| {
            let encoder: Box<dyn Write> = match format {
                Gzip => Box::new(flate2::write::GzEncoder::new(encoder, Default::default())),
                Bzip => Box::new(bzip2::write::BzEncoder::new(encoder, Default::default())),
                Lzma => Box::new(xz2::write::XzEncoder::new(encoder, 6)),
                Zstd => {
                    let zstd_encoder = zstd::stream::write::Encoder::new(encoder, Default::default());
                    // Safety:
                    //     Encoder::new() can only fail if `level` is invalid, but Default::default()
                    //     is guaranteed to be valid
                    Box::new(zstd_encoder.unwrap().auto_finish())
                }
                _ => unreachable!(),
            };
            encoder
        };

        for format in formats.iter().skip(1).rev() {
            writer = chain_writer_encoder(format, writer);
        }

        match formats[0] {
            Gzip | Bzip | Lzma | Zstd => {
                writer = chain_writer_encoder(&formats[0], writer);
                let mut reader = fs::File::open(&files[0]).unwrap();
                io::copy(&mut reader, &mut writer)?;
            }
            Tar => {
                let mut writer = archive::tar::build_archive_from_paths(&files, writer)?;
                writer.flush()?;
            }
            Tgz => {
                let encoder = flate2::write::GzEncoder::new(writer, Default::default());
                let writer = archive::tar::build_archive_from_paths(&files, encoder)?;
                writer.finish()?.flush()?;
            }
            Tbz => {
                let encoder = bzip2::write::BzEncoder::new(writer, Default::default());
                let writer = archive::tar::build_archive_from_paths(&files, encoder)?;
                writer.finish()?.flush()?;
            }
            Tlzma => {
                let encoder = xz2::write::XzEncoder::new(writer, 6);
                let writer = archive::tar::build_archive_from_paths(&files, encoder)?;
                writer.finish()?.flush()?;
            }
            Tzst => {
                let encoder = zstd::stream::write::Encoder::new(writer, Default::default())?;
                let writer = archive::tar::build_archive_from_paths(&files, encoder)?;
                writer.finish()?.flush()?;
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
    }

    Ok(())
}

// File at input_file_path is opened for reading, example: "archive.tar.gz"
// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
// output_folder it's where the file will be decompressed to
// file_name is only used when extracting single file formats, no archive formats like .tar or .zip
fn decompress_file(
    input_file_path: &Path,
    formats: Vec<extension::CompressionFormat>,
    output_folder: Option<&Path>,
    file_name: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<()> {
    // TODO: improve error message
    let reader = fs::File::open(&input_file_path)?;

    // Output path is used by single file formats
    let output_path =
        if let Some(output_folder) = output_folder { output_folder.join(file_name) } else { file_name.to_path_buf() };

    // Output folder is used by archive file formats (zip and tar)
    let output_folder = output_folder.unwrap_or_else(|| Path::new("."));

    // Zip archives are special, because they require io::Seek, so it requires it's logic separated
    // from decoder chaining.
    //
    // This is the only case where we can read and unpack it directly, without having to do
    // in-memory decompression/copying first.
    //
    // Any other Zip decompression done can take up the whole RAM and freeze ouch.
    if let [Zip] = *formats.as_slice() {
        utils::create_dir_if_non_existent(output_folder)?;
        let zip_archive = zip::ZipArchive::new(reader)?;
        let _files = crate::archive::zip::unpack_archive(zip_archive, output_folder, question_policy)?;
        info!("Successfully decompressed archive in {}.", nice_directory_display(output_folder));
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
            Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            _ => unreachable!(),
        };
        Ok(decoder)
    };

    for format in formats.iter().skip(1).rev() {
        reader = chain_reader_decoder(format, reader)?;
    }

    utils::create_dir_if_non_existent(output_folder)?;

    let files_unpacked;

    match formats[0] {
        Gzip | Bzip | Lzma | Zstd => {
            reader = chain_reader_decoder(&formats[0], reader)?;

            // TODO: improve error treatment
            let mut writer = fs::File::create(&output_path)?;

            io::copy(&mut reader, &mut writer)?;
            files_unpacked = vec![output_path];
        }
        Tar => {
            files_unpacked = crate::archive::tar::unpack_archive(reader, output_folder, question_policy)?;
        }
        Tgz => {
            let reader = chain_reader_decoder(&Gzip, reader)?;
            files_unpacked = crate::archive::tar::unpack_archive(reader, output_folder, question_policy)?;
        }
        Tbz => {
            let reader = chain_reader_decoder(&Bzip, reader)?;
            files_unpacked = crate::archive::tar::unpack_archive(reader, output_folder, question_policy)?;
        }
        Tlzma => {
            let reader = chain_reader_decoder(&Lzma, reader)?;
            files_unpacked = crate::archive::tar::unpack_archive(reader, output_folder, question_policy)?;
        }
        Tzst => {
            let reader = chain_reader_decoder(&Zstd, reader)?;
            files_unpacked = crate::archive::tar::unpack_archive(reader, output_folder, question_policy)?;
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

            files_unpacked = crate::archive::zip::unpack_archive(zip_archive, output_folder, question_policy)?;
        }
    }

    info!("Successfully decompressed archive in {}.", nice_directory_display(output_folder));
    info!("Files unpacked: {}", files_unpacked.len());

    Ok(())
}
