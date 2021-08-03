use std::{
    fs,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

use utils::colors;

use crate::{
    cli::Command,
    compressors::{
        BzipCompressor, Compressor, Entry, GzipCompressor, LzmaCompressor, TarCompressor,
        ZipCompressor,
    },
    extension::{
        self,
        CompressionFormat::{self, *},
    },
    file, oof, utils,
    utils::to_utf,
};

pub fn run(command: Command, flags: &oof::Flags) -> crate::Result<()> {
    match command {
        Command::Compress { files, compressed_output_path } => {
            compress_files(files, &compressed_output_path, flags)?
        },
        Command::Decompress { files, output_folder } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            for path in files.iter() {
                let (file_output_path, file_formats) =
                    extension::separate_known_extensions_from_name(path);
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
                decompress_file(input_path, formats, output_folder, file_name, flags)?;
            }
        },
        Command::ShowHelp => crate::help_command(),
        Command::ShowVersion => crate::version_command(),
    }
    Ok(())
}

type BoxedCompressor = Box<dyn Compressor>;

fn get_compressor(file: &file::File) -> crate::Result<(Option<BoxedCompressor>, BoxedCompressor)> {
    let extension = match &file.extension {
        Some(extension) => extension,
        None => {
            // This is reached when the output file given does not have an extension or has an unsupported one
            return Err(crate::Error::MissingExtensionError(file.path.to_path_buf()));
        },
    };

    // Supported first compressors:
    // .tar and .zip
    let first_compressor: Option<Box<dyn Compressor>> = match &extension.first_ext {
        Some(ext) => match ext {
            CompressionFormat::Tar => Some(Box::new(TarCompressor)),
            CompressionFormat::Zip => Some(Box::new(ZipCompressor)),
            CompressionFormat::Bzip => Some(Box::new(BzipCompressor)),
            CompressionFormat::Gzip => Some(Box::new(GzipCompressor)),
            CompressionFormat::Lzma => Some(Box::new(LzmaCompressor)),
        },
        None => None,
    };

    // Supported second compressors:
    // any
    let second_compressor: Box<dyn Compressor> = match extension.second_ext {
        CompressionFormat::Tar => Box::new(TarCompressor),
        CompressionFormat::Zip => Box::new(ZipCompressor),
        CompressionFormat::Bzip => Box::new(BzipCompressor),
        CompressionFormat::Gzip => Box::new(GzipCompressor),
        CompressionFormat::Lzma => Box::new(LzmaCompressor),
    };

    Ok((first_compressor, second_compressor))
}

fn compress_files(
    files: Vec<PathBuf>,
    output_path: &Path,
    flags: &oof::Flags,
) -> crate::Result<()> {
    let mut output = file::File::from(output_path)?;

    let (first_compressor, second_compressor) = get_compressor(&output)?;

    if output_path.exists() && !utils::permission_for_overwriting(output_path, flags)? {
        // The user does not want to overwrite the file
        return Ok(());
    }

    let bytes = match first_compressor {
        Some(first_compressor) => {
            let mut entry = Entry::Files(files);
            let bytes = first_compressor.compress(entry)?;

            output.contents_in_memory = Some(bytes);
            entry = Entry::InMemory(output);
            second_compressor.compress(entry)?
        },
        None => {
            let entry = Entry::Files(files);
            second_compressor.compress(entry)?
        },
    };

    println!(
        "{}[INFO]{} writing to {:?}. ({})",
        colors::yellow(),
        colors::reset(),
        output_path,
        utils::Bytes::new(bytes.len() as u64)
    );
    fs::write(output_path, bytes)?;

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
    flags: &oof::Flags,
) -> crate::Result<()> {
    // TODO: improve error message
    let reader = fs::File::open(&input_file_path)?;

    // Output path is used by single file formats
    let output_path = if let Some(output_folder) = output_folder {
        output_folder.join(file_name)
    } else {
        file_name.to_path_buf()
    };

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
        let _files = crate::archive::zip::unpack_archive(zip_archive, output_folder, flags)?;
        println!("[INFO]: Successfully uncompressed bundle at '{}'.", to_utf(output_folder));
        return Ok(());
    }

    // Will be used in decoder chaining
    let reader = BufReader::new(reader);
    let mut reader: Box<dyn Read> = Box::new(reader);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
            Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
            _ => unreachable!(),
        };
        decoder
    };

    for format in formats.iter().skip(1).rev() {
        reader = chain_reader_decoder(format, reader);
    }

    match formats[0] {
        Gzip | Bzip | Lzma => {
            reader = chain_reader_decoder(&formats[0], reader);

            // TODO: improve error treatment
            let mut writer = fs::File::create(&output_path)?;

            io::copy(&mut reader, &mut writer)?;
            println!("[INFO]: Successfully uncompressed file at '{}'.", to_utf(output_path));
        },
        Tar => {
            utils::create_dir_if_non_existent(output_folder)?;
            let _ = crate::archive::tar::unpack_archive(reader, output_folder, flags)?;
            println!("[INFO]: Successfully uncompressed bundle at '{}'.", to_utf(output_folder));
        },
        Zip => {
            utils::create_dir_if_non_existent(output_folder)?;

            eprintln!("Compressing first into .zip.");
            eprintln!("Warning: .zip archives with extra extensions have a downside.");
            eprintln!("The only way is loading everything into the RAM while compressing, and then write everything down.");
            eprintln!("this means that by compressing .zip with extra compression formats, you can run out of RAM if the file is too large!");

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            let _ = crate::archive::zip::unpack_archive(zip_archive, output_folder, flags)?;

            println!("[INFO]: Successfully uncompressed bundle at '{}'.", to_utf(output_folder));
        },
    }

    Ok(())
}
