use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use utils::colors;

use crate::{
    cli::Command,
    compressors::{
        BzipCompressor, Compressor, Entry, GzipCompressor, LzmaCompressor, TarCompressor,
        ZipCompressor,
    },
    decompressors::{
        BzipDecompressor, DecompressionResult, Decompressor, GzipDecompressor, LzmaDecompressor,
        TarDecompressor, ZipDecompressor,
    },
    dialogs::Confirmation,
    extension::{CompressionFormat, Extension},
    file::File,
    utils,
};

pub fn run(command: Command, flags: &oof::Flags) -> crate::Result<()> {
    match command {
        Command::Compress {
            files,
            compressed_output_path,
        } => compress_files(files, &compressed_output_path, flags)?,
        Command::Decompress {
            files,
            output_folder,
        } => {
            // From Option<PathBuf> to Option<&Path>
            let output_folder = output_folder.as_ref().map(|path| Path::new(path));
            for file in files.iter() {
                decompress_file(file, output_folder, flags)?;
            }
        }
        Command::ShowHelp => crate::help_command(),
        Command::ShowVersion => crate::version_command(),
    }
    Ok(())
}

type BoxedCompressor = Box<dyn Compressor>;
type BoxedDecompressor = Box<dyn Decompressor>;

fn get_compressor(file: &File) -> crate::Result<(Option<BoxedCompressor>, BoxedCompressor)> {
    let extension = match &file.extension {
        Some(extension) => extension.clone(),
        None => {
            // This is reached when the output file given does not have an extension or has an unsupported one
            return Err(crate::Error::MissingExtensionError(file.path.to_path_buf()));
        }
    };

    // Supported first compressors:
    // .tar and .zip
    let first_compressor: Option<Box<dyn Compressor>> = match extension.first_ext {
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

fn get_decompressor(file: &File) -> crate::Result<(Option<BoxedDecompressor>, BoxedDecompressor)> {
    let extension = match &file.extension {
        Some(extension) => extension.clone(),
        None => {
            // This block *should* be unreachable
            eprintln!(
                "{}[internal error]{} reached Evaluator::get_decompressor without known extension.",
                colors::red(),
                colors::reset()
            );
            return Err(crate::Error::InvalidInput);
        }
    };

    let second_decompressor: Box<dyn Decompressor> = match extension.second_ext {
        CompressionFormat::Tar => Box::new(TarDecompressor),
        CompressionFormat::Zip => Box::new(ZipDecompressor),
        CompressionFormat::Gzip => Box::new(GzipDecompressor),
        CompressionFormat::Lzma => Box::new(LzmaDecompressor),
        CompressionFormat::Bzip => Box::new(BzipDecompressor),
    };

    let first_decompressor: Option<Box<dyn Decompressor>> = match extension.first_ext {
        Some(ext) => match ext {
            CompressionFormat::Tar => Some(Box::new(TarDecompressor)),
            CompressionFormat::Zip => Some(Box::new(ZipDecompressor)),
            _other => None,
        },
        None => None,
    };

    Ok((first_decompressor, second_decompressor))
}

fn decompress_file_in_memory(
    bytes: Vec<u8>,
    file_path: &Path,
    decompressor: Option<Box<dyn Decompressor>>,
    output_file: Option<File>,
    extension: Option<Extension>,
    flags: &oof::Flags,
) -> crate::Result<()> {
    let output_file_path = utils::get_destination_path(&output_file);

    let file_name = file_path
        .file_stem()
        .map(Path::new)
        .unwrap_or(output_file_path);

    if "." == file_name.as_os_str() {
        // I believe this is only possible when the supplied input has a name
        // of the sort `.tar` or `.zip' and no output has been supplied.
        // file_name = OsStr::new("ouch-output");
        todo!("Pending review, what is this supposed to do??");
    }

    // If there is a decompressor to use, we'll create a file in-memory and decompress it
    let decompressor = match decompressor {
        Some(decompressor) => decompressor,
        None => {
            // There is no more processing to be done on the input file (or there is but currently unsupported)
            // Therefore, we'll save what we have in memory into a file.
            println!("{}[INFO]{} saving to {:?}.", colors::yellow(), colors::reset(), file_name);

            if file_name.exists() {
                let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));
                if !utils::permission_for_overwriting(&file_name, flags, &confirm)? {
                    return Ok(());
                }
            }

            let mut f = fs::File::create(output_file_path.join(file_name))?;
            f.write_all(&bytes)?;
            return Ok(());
        }
    };

    let file = File {
        path: file_name,
        contents_in_memory: Some(bytes),
        extension,
    };

    let decompression_result = decompressor.decompress(file, &output_file, flags)?;
    if let DecompressionResult::FileInMemory(_) = decompression_result {
        unreachable!("Shouldn't");
    }

    Ok(())
}

fn compress_files(
    files: Vec<PathBuf>,
    output_path: &Path,
    flags: &oof::Flags,
) -> crate::Result<()> {
    let mut output = File::from(output_path)?;

    let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));
    let (first_compressor, second_compressor) = get_compressor(&output)?;

    if output_path.exists() && !utils::permission_for_overwriting(&output_path, flags, &confirm)? {
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
        }
        None => {
            let entry = Entry::Files(files);
            second_compressor.compress(entry)?
        }
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

fn decompress_file(
    file_path: &Path,
    output: Option<&Path>,
    flags: &oof::Flags,
) -> crate::Result<()> {
    // The file to be decompressed
    let file = File::from(file_path)?;
    // The file must have a supported decompressible format
    if file.extension == None {
        return Err(crate::Error::MissingExtensionError(PathBuf::from(
            file_path,
        )));
    }

    let output = match output {
        Some(inner) => Some(File::from(inner)?),
        None => None,
    };
    let (first_decompressor, second_decompressor) = get_decompressor(&file)?;

    let extension = file.extension.clone();

    let decompression_result = second_decompressor.decompress(file, &output, &flags)?;

    match decompression_result {
        DecompressionResult::FileInMemory(bytes) => {
            // We'll now decompress a file currently in memory.
            // This will currently happen in the case of .bz, .xz and .lzma
            decompress_file_in_memory(
                bytes,
                file_path,
                first_decompressor,
                output,
                extension,
                flags,
            )?;
        }
        DecompressionResult::FilesUnpacked(_files) => {
            // If the file's last extension was an archival method,
            // such as .tar, .zip or (to-do) .rar, then we won't look for
            // further processing.
            // The reason for this is that cases such as "file.xz.tar" are too rare
            // to worry about, at least at the moment.

            // TODO: use the `files` variable for something
        }
    }

    Ok(())
}
