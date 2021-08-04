use std::{
    fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use utils::colors;

use crate::{
    archive,
    cli::Command,
    extension::{
        self,
        CompressionFormat::{self, *},
    },
    oof, utils,
    utils::to_utf,
};

pub fn run(command: Command, flags: &oof::Flags) -> crate::Result<()> {
    match command {
        Command::Compress { files, output_path } => {
            let formats = extension::extensions_from_path(&output_path);
            let output_file = fs::File::create(&output_path)?;
            let compression_result = compress_files(files, formats, output_file, flags);
            if let Err(_err) = compression_result {
                fs::remove_file(&output_path).unwrap();
            }
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

fn compress_files(
    files: Vec<PathBuf>,
    formats: Vec<CompressionFormat>,
    output_file: fs::File,
    _flags: &oof::Flags,
) -> crate::Result<()> {
    let file_writer = BufWriter::new(output_file);

    if formats.len() == 1 {
        let build_archive_from_paths = match formats[0] {
            Tar => archive::tar::build_archive_from_paths,
            Zip => archive::zip::build_archive_from_paths,
            _ => unreachable!(),
        };

        let mut bufwriter = build_archive_from_paths(&files, file_writer)?;
        bufwriter.flush()?;
    } else {
        let mut writer: Box<dyn Write> = Box::new(file_writer);

        // Grab previous encoder and wrap it inside of a new one
        let chain_writer_encoder = |format: &CompressionFormat, encoder: Box<dyn Write>| {
            let encoder: Box<dyn Write> = match format {
                Gzip => Box::new(flate2::write::GzEncoder::new(encoder, Default::default())),
                Bzip => Box::new(bzip2::write::BzEncoder::new(encoder, Default::default())),
                Lzma => Box::new(xz2::write::XzEncoder::new(encoder, 6)),
                _ => unreachable!(),
            };
            encoder
        };

        for format in formats.iter().skip(1).rev() {
            writer = chain_writer_encoder(format, writer);
        }

        match formats[0] {
            Gzip | Bzip | Lzma => {
                writer = chain_writer_encoder(&formats[0], writer);
                let mut reader = fs::File::open(&files[0]).unwrap();
                io::copy(&mut reader, &mut writer)?;
            },
            Tar => {
                let mut writer = archive::tar::build_archive_from_paths(&files, writer)?;
                writer.flush()?;
            },
            Zip => {
                eprintln!(
                    "{yellow}Warning:{reset}",
                    yellow = colors::yellow(),
                    reset = colors::reset()
                );
                eprintln!("\tCompressing .zip entirely in memory.");
                eprintln!("\tIf the file is too big, your pc might freeze!");
                eprintln!(
                    "\tThis is a limitation for formats like '{}'.",
                    formats.iter().map(|format| format.to_string()).collect::<String>()
                );
                eprintln!("\tThe design of .zip makes it impossible to compress via stream.");

                let mut vec_buffer = io::Cursor::new(vec![]);
                archive::zip::build_archive_from_paths(&files, &mut vec_buffer)?;
                let vec_buffer = vec_buffer.into_inner();
                io::copy(&mut vec_buffer.as_slice(), &mut writer)?;
            },
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
