use std::{
    io::{self, BufReader, Read},
    path::Path,
};

use fs_err as fs;

use crate::{
    commands::warn_user_about_loading_zip_in_memory,
    extension::CompressionFormat::{self, *},
    list::{self, FileInArchive, ListOptions},
    utils::user_wants_to_continue,
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};
use crate::archive::sevenz;

/// File at input_file_path is opened for reading, example: "archive.tar.gz"
/// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
pub fn list_archive_contents(
    archive_path: &Path,
    formats: Vec<CompressionFormat>,
    list_options: ListOptions,
    question_policy: QuestionPolicy,
    password: Option<&str>,
) -> crate::Result<()> {
    let reader = fs::File::open(archive_path)?;

    // Zip archives are special, because they require io::Seek, so it requires it's logic separated
    // from decoder chaining.
    //
    // This is the only case where we can read and unpack it directly, without having to do
    // in-memory decompression/copying first.
    //
    // Any other Zip decompression done can take up the whole RAM and freeze ouch.
    if let &[Zip] = formats.as_slice() {
        let zip_archive = zip::ZipArchive::new(reader)?;
        let files = crate::archive::zip::list_archive(zip_archive, password);
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
                Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(decoder)),
                Lzma => Box::new(xz2::read::XzDecoder::new(decoder)),
                Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
                Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
                Tar | Zip | Rar | SevenZip => unreachable!(),
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
                warn_user_about_loading_zip_in_memory();

                if !user_wants_to_continue(archive_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;
            let zip_archive = zip::ZipArchive::new(io::Cursor::new(vec))?;

            Box::new(crate::archive::zip::list_archive(zip_archive, password))
        }
        #[cfg(feature = "unrar")]
        Rar => {
            if formats.len() > 1 {
                let mut temp_file = tempfile::NamedTempFile::new()?;
                io::copy(&mut reader, &mut temp_file)?;
                Box::new(crate::archive::rar::list_archive(temp_file.path(), password))
            } else {
                Box::new(crate::archive::rar::list_archive(archive_path, password))
            }
        }
        #[cfg(not(feature = "unrar"))]
        Rar => {
            return Err(crate::archive::rar_stub::no_support());
        }
        SevenZip => {
            if formats.len() > 1 {
                warn_user_about_loading_zip_in_memory();
                if !user_wants_to_continue(archive_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            Box::new(sevenz::list_archive(archive_path, password))
        }
        Gzip | Bzip | Lz4 | Lzma | Snappy | Zstd => {
            panic!("Not an archive! This should never happen, if it does, something is wrong with `CompressionFormat::is_archive()`. Please report this error!");
        }
    };
    list::list_files(archive_path, files, list_options)?;
    Ok(())
}
