use std::{
    io::{self, BufReader, Read},
    path::Path,
};

use fs_err as fs;

use crate::{
    archive,
    commands::warn_user_about_loading_zip_in_memory,
    extension::CompressionFormat::{self, *},
    list::{self, FileInArchive, ListOptions},
    utils::{io::lock_and_flush_output_stdio, user_wants_to_continue},
    QuestionAction, QuestionPolicy, BUFFER_CAPACITY,
};

/// File at input_file_path is opened for reading, example: "archive.tar.gz"
/// formats contains each format necessary for decompression, example: [Gz, Tar] (in decompression order)
pub fn list_archive_contents(
    archive_path: &Path,
    formats: Vec<CompressionFormat>,
    list_options: ListOptions,
    question_policy: QuestionPolicy,
    password: Option<&[u8]>,
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
        |format: CompressionFormat, decoder: Box<dyn Read + Send>| -> crate::Result<Box<dyn Read + Send>> {
            let decoder: Box<dyn Read + Send> = match format {
                Gzip => Box::new(flate2::read::GzDecoder::new(decoder)),
                Bzip => Box::new(bzip2::read::BzDecoder::new(decoder)),
                Bzip3 => {
                    #[cfg(not(feature = "bzip3"))]
                    return Err(archive::bzip3_stub::no_support());

                    #[cfg(feature = "bzip3")]
                    Box::new(bzip3::read::Bz3Decoder::new(decoder).unwrap())
                }
                Lz4 => Box::new(lz4_flex::frame::FrameDecoder::new(decoder)),
                Lzma => Box::new(lzma_rust2::LzmaReader::new_mem_limit(decoder, u32::MAX, None)?),
                Xz => Box::new(lzma_rust2::XzReader::new(decoder, true)),
                Lzip => Box::new(lzma_rust2::LzipReader::new(decoder)?),
                Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
                Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
                Brotli => Box::new(brotli::Decompressor::new(decoder, BUFFER_CAPACITY)),
                Tar | Zip | Rar | SevenZip => unreachable!("should be treated by caller"),
            };
            Ok(decoder)
        };

    let mut misplaced_archive_format = None;
    for &format in formats.iter().skip(1).rev() {
        if format.archive_format() {
            misplaced_archive_format = Some(format);
            break;
        }
        reader = chain_reader_decoder(format, reader)?;
    }

    let archive_format = misplaced_archive_format.unwrap_or(formats[0]);
    let files: Box<dyn Iterator<Item = crate::Result<FileInArchive>>> = match archive_format {
        Tar => Box::new(crate::archive::tar::list_archive(tar::Archive::new(reader))),
        Zip => {
            if formats.len() > 1 {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

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
                Box::new(crate::archive::rar::list_archive(temp_file.path(), password)?)
            } else {
                Box::new(crate::archive::rar::list_archive(archive_path, password)?)
            }
        }
        #[cfg(not(feature = "unrar"))]
        Rar => {
            return Err(crate::archive::rar_stub::no_support());
        }
        SevenZip => {
            if formats.len() > 1 {
                // Locking necessary to guarantee that warning and question
                // messages stay adjacent
                let _locks = lock_and_flush_output_stdio();

                warn_user_about_loading_zip_in_memory();
                if !user_wants_to_continue(archive_path, question_policy, QuestionAction::Decompression)? {
                    return Ok(());
                }
            }

            let mut vec = vec![];
            io::copy(&mut reader, &mut vec)?;

            Box::new(archive::sevenz::list_archive(io::Cursor::new(vec), password)?)
        }
        Gzip | Bzip | Bzip3 | Lz4 | Lzma | Xz | Lzip | Snappy | Zstd | Brotli => {
            unreachable!("Not an archive, should be validated before calling this function.");
        }
    };

    list::list_files(archive_path, files, list_options)
}
