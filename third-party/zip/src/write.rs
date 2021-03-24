//! Types for creating ZIP archives

use crate::compression::CompressionMethod;
use crate::read::ZipFile;
use crate::result::{ZipError, ZipResult};
use crate::spec;
use crate::types::{DateTime, System, ZipFileData, DEFAULT_VERSION};
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use std::default::Default;
use std::io;
use std::io::prelude::*;
use std::mem;

#[cfg(any(
    feature = "deflate",
    feature = "deflate-miniz",
    feature = "deflate-zlib"
))]
use flate2::write::DeflateEncoder;

#[cfg(feature = "bzip2")]
use bzip2::write::BzEncoder;

enum GenericZipWriter<W: Write + io::Seek> {
    Closed,
    Storer(W),
    #[cfg(any(
        feature = "deflate",
        feature = "deflate-miniz",
        feature = "deflate-zlib"
    ))]
    Deflater(DeflateEncoder<W>),
    #[cfg(feature = "bzip2")]
    Bzip2(BzEncoder<W>),
}

/// ZIP archive generator
///
/// Handles the bookkeeping involved in building an archive, and provides an
/// API to edit its contents.
///
/// ```
/// # fn doit() -> zip::result::ZipResult<()>
/// # {
/// # use zip::ZipWriter;
/// use std::io::Write;
/// use zip::write::FileOptions;
///
/// // We use a buffer here, though you'd normally use a `File`
/// let mut buf = [0; 65536];
/// let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf[..]));
///
/// let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
/// zip.start_file("hello_world.txt", options)?;
/// zip.write(b"Hello, World!")?;
///
/// // Apply the changes you've made.
/// // Dropping the `ZipWriter` will have the same effect, but may silently fail
/// zip.finish()?;
///
/// # Ok(())
/// # }
/// # doit().unwrap();
/// ```
pub struct ZipWriter<W: Write + io::Seek> {
    inner: GenericZipWriter<W>,
    files: Vec<ZipFileData>,
    stats: ZipWriterStats,
    writing_to_file: bool,
    comment: String,
    writing_raw: bool,
}

#[derive(Default)]
struct ZipWriterStats {
    hasher: Hasher,
    start: u64,
    bytes_written: u64,
}

struct ZipRawValues {
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
}

/// Metadata for a file to be written
#[derive(Copy, Clone)]
pub struct FileOptions {
    compression_method: CompressionMethod,
    last_modified_time: DateTime,
    permissions: Option<u32>,
}

impl FileOptions {
    /// Construct a new FileOptions object
    pub fn default() -> FileOptions {
        FileOptions {
            #[cfg(any(
                feature = "deflate",
                feature = "deflate-miniz",
                feature = "deflate-zlib"
            ))]
            compression_method: CompressionMethod::Deflated,
            #[cfg(not(any(
                feature = "deflate",
                feature = "deflate-miniz",
                feature = "deflate-zlib"
            )))]
            compression_method: CompressionMethod::Stored,
            #[cfg(feature = "time")]
            last_modified_time: DateTime::from_time(time::now()).unwrap_or_default(),
            #[cfg(not(feature = "time"))]
            last_modified_time: DateTime::default(),
            permissions: None,
        }
    }

    /// Set the compression method for the new file
    ///
    /// The default is `CompressionMethod::Deflated`. If the deflate compression feature is
    /// disabled, `CompressionMethod::Stored` becomes the default.
    /// otherwise.
    pub fn compression_method(mut self, method: CompressionMethod) -> FileOptions {
        self.compression_method = method;
        self
    }

    /// Set the last modified time
    ///
    /// The default is the current timestamp if the 'time' feature is enabled, and 1980-01-01
    /// otherwise
    pub fn last_modified_time(mut self, mod_time: DateTime) -> FileOptions {
        self.last_modified_time = mod_time;
        self
    }

    /// Set the permissions for the new file.
    ///
    /// The format is represented with unix-style permissions.
    /// The default is `0o644`, which represents `rw-r--r--` for files,
    /// and `0o755`, which represents `rwxr-xr-x` for directories
    pub fn unix_permissions(mut self, mode: u32) -> FileOptions {
        self.permissions = Some(mode & 0o777);
        self
    }
}

impl Default for FileOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl<W: Write + io::Seek> Write for ZipWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.writing_to_file {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No file has been started",
            ));
        }
        match self.inner.ref_mut() {
            Some(ref mut w) => {
                let write_result = w.write(buf);
                if let Ok(count) = write_result {
                    self.stats.update(&buf[0..count]);
                }
                write_result
            }
            None => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "ZipWriter was already closed",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.ref_mut() {
            Some(ref mut w) => w.flush(),
            None => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "ZipWriter was already closed",
            )),
        }
    }
}

impl ZipWriterStats {
    fn update(&mut self, buf: &[u8]) {
        self.hasher.update(buf);
        self.bytes_written += buf.len() as u64;
    }
}

impl<W: Write + io::Seek> ZipWriter<W> {
    /// Initializes the archive.
    ///
    /// Before writing to this object, the [`ZipWriter::start_file`] function should be called.
    pub fn new(inner: W) -> ZipWriter<W> {
        ZipWriter {
            inner: GenericZipWriter::Storer(inner),
            files: Vec::new(),
            stats: Default::default(),
            writing_to_file: false,
            comment: String::new(),
            writing_raw: false,
        }
    }

    /// Set ZIP archive comment.
    pub fn set_comment<S>(&mut self, comment: S)
    where
        S: Into<String>,
    {
        self.comment = comment.into();
    }

    /// Start a new file for with the requested options.
    fn start_entry<S>(
        &mut self,
        name: S,
        options: FileOptions,
        raw_values: Option<ZipRawValues>,
    ) -> ZipResult<()>
    where
        S: Into<String>,
    {
        self.finish_file()?;

        let is_raw = raw_values.is_some();
        let raw_values = raw_values.unwrap_or_else(|| ZipRawValues {
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
        });

        {
            let writer = self.inner.get_plain();
            let header_start = writer.seek(io::SeekFrom::Current(0))?;

            let permissions = options.permissions.unwrap_or(0o100644);
            let mut file = ZipFileData {
                system: System::Unix,
                version_made_by: DEFAULT_VERSION,
                encrypted: false,
                compression_method: options.compression_method,
                last_modified_time: options.last_modified_time,
                crc32: raw_values.crc32,
                compressed_size: raw_values.compressed_size,
                uncompressed_size: raw_values.uncompressed_size,
                file_name: name.into(),
                file_name_raw: Vec::new(), // Never used for saving
                file_comment: String::new(),
                header_start,
                data_start: 0,
                central_header_start: 0,
                external_attributes: permissions << 16,
            };
            write_local_file_header(writer, &file)?;

            let header_end = writer.seek(io::SeekFrom::Current(0))?;
            self.stats.start = header_end;
            file.data_start = header_end;

            self.stats.bytes_written = 0;
            self.stats.hasher = Hasher::new();

            self.files.push(file);
        }

        self.writing_raw = is_raw;
        self.inner.switch_to(if is_raw {
            CompressionMethod::Stored
        } else {
            options.compression_method
        })?;

        Ok(())
    }

    fn finish_file(&mut self) -> ZipResult<()> {
        self.inner.switch_to(CompressionMethod::Stored)?;
        let writer = self.inner.get_plain();

        if !self.writing_raw {
            let file = match self.files.last_mut() {
                None => return Ok(()),
                Some(f) => f,
            };
            file.crc32 = self.stats.hasher.clone().finalize();
            file.uncompressed_size = self.stats.bytes_written;

            let file_end = writer.seek(io::SeekFrom::Current(0))?;
            file.compressed_size = file_end - self.stats.start;

            update_local_file_header(writer, file)?;
            writer.seek(io::SeekFrom::Start(file_end))?;
        }

        self.writing_to_file = false;
        self.writing_raw = false;
        Ok(())
    }

    /// Create a file in the archive and start writing its' contents.
    ///
    /// The data should be written using the [`io::Write`] implementation on this [`ZipWriter`]
    pub fn start_file<S>(&mut self, name: S, mut options: FileOptions) -> ZipResult<()>
    where
        S: Into<String>,
    {
        if options.permissions.is_none() {
            options.permissions = Some(0o644);
        }
        *options.permissions.as_mut().unwrap() |= 0o100000;
        self.start_entry(name, options, None)?;
        self.writing_to_file = true;
        Ok(())
    }

    /// Starts a file, taking a Path as argument.
    ///
    /// This function ensures that the '/' path seperator is used. It also ignores all non 'Normal'
    /// Components, such as a starting '/' or '..' and '.'.
    #[deprecated(
        since = "0.5.7",
        note = "by stripping `..`s from the path, the meaning of paths can change. Use `start_file` instead."
    )]
    pub fn start_file_from_path(
        &mut self,
        path: &std::path::Path,
        options: FileOptions,
    ) -> ZipResult<()> {
        self.start_file(path_to_string(path), options)
    }

    /// Add a new file using the already compressed data from a ZIP file being read and renames it, this
    /// allows faster copies of the `ZipFile` since there is no need to decompress and compress it again.
    /// Any `ZipFile` metadata is copied and not checked, for example the file CRC.

    /// ```no_run
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write};
    /// use zip::{ZipArchive, ZipWriter};
    ///
    /// fn copy_rename<R, W>(
    ///     src: &mut ZipArchive<R>,
    ///     dst: &mut ZipWriter<W>,
    /// ) -> zip::result::ZipResult<()>
    /// where
    ///     R: Read + Seek,
    ///     W: Write + Seek,
    /// {
    ///     // Retrieve file entry by name
    ///     let file = src.by_name("src_file.txt")?;
    ///
    ///     // Copy and rename the previously obtained file entry to the destination zip archive
    ///     dst.raw_copy_file_rename(file, "new_name.txt")?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn raw_copy_file_rename<S>(&mut self, mut file: ZipFile, name: S) -> ZipResult<()>
    where
        S: Into<String>,
    {
        let options = FileOptions::default()
            .last_modified_time(file.last_modified())
            .compression_method(file.compression());
        if let Some(perms) = file.unix_mode() {
            options.unix_permissions(perms);
        }

        let raw_values = ZipRawValues {
            crc32: file.crc32(),
            compressed_size: file.compressed_size(),
            uncompressed_size: file.size(),
        };

        self.start_entry(name, options, Some(raw_values))?;
        self.writing_to_file = true;

        io::copy(file.get_raw_reader(), self)?;

        Ok(())
    }

    /// Add a new file using the already compressed data from a ZIP file being read, this allows faster
    /// copies of the `ZipFile` since there is no need to decompress and compress it again. Any `ZipFile`
    /// metadata is copied and not checked, for example the file CRC.
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write};
    /// use zip::{ZipArchive, ZipWriter};
    ///
    /// fn copy<R, W>(src: &mut ZipArchive<R>, dst: &mut ZipWriter<W>) -> zip::result::ZipResult<()>
    /// where
    ///     R: Read + Seek,
    ///     W: Write + Seek,
    /// {
    ///     // Retrieve file entry by name
    ///     let file = src.by_name("src_file.txt")?;
    ///
    ///     // Copy the previously obtained file entry to the destination zip archive
    ///     dst.raw_copy_file(file)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn raw_copy_file(&mut self, file: ZipFile) -> ZipResult<()> {
        let name = file.name().to_owned();
        self.raw_copy_file_rename(file, name)
    }

    /// Add a directory entry.
    ///
    /// You can't write data to the file afterwards.
    pub fn add_directory<S>(&mut self, name: S, mut options: FileOptions) -> ZipResult<()>
    where
        S: Into<String>,
    {
        if options.permissions.is_none() {
            options.permissions = Some(0o755);
        }
        *options.permissions.as_mut().unwrap() |= 0o40000;
        options.compression_method = CompressionMethod::Stored;

        let name_as_string = name.into();
        // Append a slash to the filename if it does not end with it.
        let name_with_slash = match name_as_string.chars().last() {
            Some('/') | Some('\\') => name_as_string,
            _ => name_as_string + "/",
        };

        self.start_entry(name_with_slash, options, None)?;
        self.writing_to_file = false;
        Ok(())
    }

    /// Add a directory entry, taking a Path as argument.
    ///
    /// This function ensures that the '/' path seperator is used. It also ignores all non 'Normal'
    /// Components, such as a starting '/' or '..' and '.'.
    #[deprecated(
        since = "0.5.7",
        note = "by stripping `..`s from the path, the meaning of paths can change. Use `add_directory` instead."
    )]
    pub fn add_directory_from_path(
        &mut self,
        path: &std::path::Path,
        options: FileOptions,
    ) -> ZipResult<()> {
        self.add_directory(path_to_string(path), options)
    }

    /// Finish the last file and write all other zip-structures
    ///
    /// This will return the writer, but one should normally not append any data to the end of the file.
    /// Note that the zipfile will also be finished on drop.
    pub fn finish(&mut self) -> ZipResult<W> {
        self.finalize()?;
        let inner = mem::replace(&mut self.inner, GenericZipWriter::Closed);
        Ok(inner.unwrap())
    }

    fn finalize(&mut self) -> ZipResult<()> {
        self.finish_file()?;

        {
            let writer = self.inner.get_plain();

            let central_start = writer.seek(io::SeekFrom::Current(0))?;
            for file in self.files.iter() {
                write_central_directory_header(writer, file)?;
            }
            let central_size = writer.seek(io::SeekFrom::Current(0))? - central_start;

            let footer = spec::CentralDirectoryEnd {
                disk_number: 0,
                disk_with_central_directory: 0,
                number_of_files_on_this_disk: self.files.len() as u16,
                number_of_files: self.files.len() as u16,
                central_directory_size: central_size as u32,
                central_directory_offset: central_start as u32,
                zip_file_comment: self.comment.as_bytes().to_vec(),
            };

            footer.write(writer)?;
        }

        Ok(())
    }
}

impl<W: Write + io::Seek> Drop for ZipWriter<W> {
    fn drop(&mut self) {
        if !self.inner.is_closed() {
            if let Err(e) = self.finalize() {
                let _ = write!(&mut io::stderr(), "ZipWriter drop failed: {:?}", e);
            }
        }
    }
}

impl<W: Write + io::Seek> GenericZipWriter<W> {
    fn switch_to(&mut self, compression: CompressionMethod) -> ZipResult<()> {
        match self.current_compression() {
            Some(method) if method == compression => return Ok(()),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "ZipWriter was already closed",
                )
                .into())
            }
            _ => {}
        }

        let bare = match mem::replace(self, GenericZipWriter::Closed) {
            GenericZipWriter::Storer(w) => w,
            #[cfg(any(
                feature = "deflate",
                feature = "deflate-miniz",
                feature = "deflate-zlib"
            ))]
            GenericZipWriter::Deflater(w) => w.finish()?,
            #[cfg(feature = "bzip2")]
            GenericZipWriter::Bzip2(w) => w.finish()?,
            GenericZipWriter::Closed => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "ZipWriter was already closed",
                )
                .into())
            }
        };

        *self = {
            #[allow(deprecated)]
            match compression {
                CompressionMethod::Stored => GenericZipWriter::Storer(bare),
                #[cfg(any(
                    feature = "deflate",
                    feature = "deflate-miniz",
                    feature = "deflate-zlib"
                ))]
                CompressionMethod::Deflated => GenericZipWriter::Deflater(DeflateEncoder::new(
                    bare,
                    flate2::Compression::default(),
                )),
                #[cfg(feature = "bzip2")]
                CompressionMethod::Bzip2 => {
                    GenericZipWriter::Bzip2(BzEncoder::new(bare, bzip2::Compression::Default))
                }
                CompressionMethod::Unsupported(..) => {
                    return Err(ZipError::UnsupportedArchive("Unsupported compression"))
                }
            }
        };

        Ok(())
    }

    fn ref_mut(&mut self) -> Option<&mut dyn Write> {
        match *self {
            GenericZipWriter::Storer(ref mut w) => Some(w as &mut dyn Write),
            #[cfg(any(
                feature = "deflate",
                feature = "deflate-miniz",
                feature = "deflate-zlib"
            ))]
            GenericZipWriter::Deflater(ref mut w) => Some(w as &mut dyn Write),
            #[cfg(feature = "bzip2")]
            GenericZipWriter::Bzip2(ref mut w) => Some(w as &mut dyn Write),
            GenericZipWriter::Closed => None,
        }
    }

    fn is_closed(&self) -> bool {
        match *self {
            GenericZipWriter::Closed => true,
            _ => false,
        }
    }

    fn get_plain(&mut self) -> &mut W {
        match *self {
            GenericZipWriter::Storer(ref mut w) => w,
            _ => panic!("Should have switched to stored beforehand"),
        }
    }

    fn current_compression(&self) -> Option<CompressionMethod> {
        match *self {
            GenericZipWriter::Storer(..) => Some(CompressionMethod::Stored),
            #[cfg(any(
                feature = "deflate",
                feature = "deflate-miniz",
                feature = "deflate-zlib"
            ))]
            GenericZipWriter::Deflater(..) => Some(CompressionMethod::Deflated),
            #[cfg(feature = "bzip2")]
            GenericZipWriter::Bzip2(..) => Some(CompressionMethod::Bzip2),
            GenericZipWriter::Closed => None,
        }
    }

    fn unwrap(self) -> W {
        match self {
            GenericZipWriter::Storer(w) => w,
            _ => panic!("Should have switched to stored beforehand"),
        }
    }
}

fn write_local_file_header<T: Write>(writer: &mut T, file: &ZipFileData) -> ZipResult<()> {
    // local file header signature
    writer.write_u32::<LittleEndian>(spec::LOCAL_FILE_HEADER_SIGNATURE)?;
    // version needed to extract
    writer.write_u16::<LittleEndian>(file.version_needed())?;
    // general purpose bit flag
    let flag = if !file.file_name.is_ascii() {
        1u16 << 11
    } else {
        0
    };
    writer.write_u16::<LittleEndian>(flag)?;
    // Compression method
    #[allow(deprecated)]
    writer.write_u16::<LittleEndian>(file.compression_method.to_u16())?;
    // last mod file time and last mod file date
    writer.write_u16::<LittleEndian>(file.last_modified_time.timepart())?;
    writer.write_u16::<LittleEndian>(file.last_modified_time.datepart())?;
    // crc-32
    writer.write_u32::<LittleEndian>(file.crc32)?;
    // compressed size
    writer.write_u32::<LittleEndian>(file.compressed_size as u32)?;
    // uncompressed size
    writer.write_u32::<LittleEndian>(file.uncompressed_size as u32)?;
    // file name length
    writer.write_u16::<LittleEndian>(file.file_name.as_bytes().len() as u16)?;
    // extra field length
    let extra_field = build_extra_field(file)?;
    writer.write_u16::<LittleEndian>(extra_field.len() as u16)?;
    // file name
    writer.write_all(file.file_name.as_bytes())?;
    // extra field
    writer.write_all(&extra_field)?;

    Ok(())
}

fn update_local_file_header<T: Write + io::Seek>(
    writer: &mut T,
    file: &ZipFileData,
) -> ZipResult<()> {
    const CRC32_OFFSET: u64 = 14;
    writer.seek(io::SeekFrom::Start(file.header_start + CRC32_OFFSET))?;
    writer.write_u32::<LittleEndian>(file.crc32)?;
    writer.write_u32::<LittleEndian>(file.compressed_size as u32)?;
    writer.write_u32::<LittleEndian>(file.uncompressed_size as u32)?;
    Ok(())
}

fn write_central_directory_header<T: Write>(writer: &mut T, file: &ZipFileData) -> ZipResult<()> {
    // central file header signature
    writer.write_u32::<LittleEndian>(spec::CENTRAL_DIRECTORY_HEADER_SIGNATURE)?;
    // version made by
    let version_made_by = (file.system as u16) << 8 | (file.version_made_by as u16);
    writer.write_u16::<LittleEndian>(version_made_by)?;
    // version needed to extract
    writer.write_u16::<LittleEndian>(file.version_needed())?;
    // general puprose bit flag
    let flag = if !file.file_name.is_ascii() {
        1u16 << 11
    } else {
        0
    };
    writer.write_u16::<LittleEndian>(flag)?;
    // compression method
    #[allow(deprecated)]
    writer.write_u16::<LittleEndian>(file.compression_method.to_u16())?;
    // last mod file time + date
    writer.write_u16::<LittleEndian>(file.last_modified_time.timepart())?;
    writer.write_u16::<LittleEndian>(file.last_modified_time.datepart())?;
    // crc-32
    writer.write_u32::<LittleEndian>(file.crc32)?;
    // compressed size
    writer.write_u32::<LittleEndian>(file.compressed_size as u32)?;
    // uncompressed size
    writer.write_u32::<LittleEndian>(file.uncompressed_size as u32)?;
    // file name length
    writer.write_u16::<LittleEndian>(file.file_name.as_bytes().len() as u16)?;
    // extra field length
    let extra_field = build_extra_field(file)?;
    writer.write_u16::<LittleEndian>(extra_field.len() as u16)?;
    // file comment length
    writer.write_u16::<LittleEndian>(0)?;
    // disk number start
    writer.write_u16::<LittleEndian>(0)?;
    // internal file attribytes
    writer.write_u16::<LittleEndian>(0)?;
    // external file attributes
    writer.write_u32::<LittleEndian>(file.external_attributes)?;
    // relative offset of local header
    writer.write_u32::<LittleEndian>(file.header_start as u32)?;
    // file name
    writer.write_all(file.file_name.as_bytes())?;
    // extra field
    writer.write_all(&extra_field)?;
    // file comment
    // <none>

    Ok(())
}

fn build_extra_field(_file: &ZipFileData) -> ZipResult<Vec<u8>> {
    let writer = Vec::new();
    // Future work
    Ok(writer)
}

fn path_to_string(path: &std::path::Path) -> String {
    let mut path_str = String::new();
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if !path_str.is_empty() {
                path_str.push('/');
            }
            path_str.push_str(&*os_str.to_string_lossy());
        }
    }
    path_str
}

#[cfg(test)]
mod test {
    use super::{FileOptions, ZipWriter};
    use crate::compression::CompressionMethod;
    use crate::types::DateTime;
    use std::io;
    use std::io::Write;

    #[test]
    fn write_empty_zip() {
        let mut writer = ZipWriter::new(io::Cursor::new(Vec::new()));
        writer.set_comment("ZIP");
        let result = writer.finish().unwrap();
        assert_eq!(result.get_ref().len(), 25);
        assert_eq!(
            *result.get_ref(),
            [80, 75, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 90, 73, 80]
        );
    }

    #[test]
    fn write_zip_dir() {
        let mut writer = ZipWriter::new(io::Cursor::new(Vec::new()));
        writer
            .add_directory(
                "test",
                FileOptions::default().last_modified_time(
                    DateTime::from_date_and_time(2018, 8, 15, 20, 45, 6).unwrap(),
                ),
            )
            .unwrap();
        assert!(writer
            .write(b"writing to a directory is not allowed, and will not write any data")
            .is_err());
        let result = writer.finish().unwrap();
        assert_eq!(result.get_ref().len(), 108);
        assert_eq!(
            *result.get_ref(),
            &[
                80u8, 75, 3, 4, 20, 0, 0, 0, 0, 0, 163, 165, 15, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 5, 0, 0, 0, 116, 101, 115, 116, 47, 80, 75, 1, 2, 46, 3, 20, 0, 0, 0, 0, 0,
                163, 165, 15, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 237, 65, 0, 0, 0, 0, 116, 101, 115, 116, 47, 80, 75, 5, 6, 0, 0, 0, 0, 1, 0,
                1, 0, 51, 0, 0, 0, 35, 0, 0, 0, 0, 0,
            ] as &[u8]
        );
    }

    #[test]
    fn write_mimetype_zip() {
        let mut writer = ZipWriter::new(io::Cursor::new(Vec::new()));
        let options = FileOptions {
            compression_method: CompressionMethod::Stored,
            last_modified_time: DateTime::default(),
            permissions: Some(33188),
        };
        writer.start_file("mimetype", options).unwrap();
        writer
            .write(b"application/vnd.oasis.opendocument.text")
            .unwrap();
        let result = writer.finish().unwrap();

        assert_eq!(result.get_ref().len(), 153);
        let mut v = Vec::new();
        v.extend_from_slice(include_bytes!("../tests/data/mimetype.zip"));
        assert_eq!(result.get_ref(), &v);
    }

    #[test]
    fn path_to_string() {
        let mut path = std::path::PathBuf::new();
        #[cfg(windows)]
        path.push(r"C:\");
        #[cfg(unix)]
        path.push("/");
        path.push("windows");
        path.push("..");
        path.push(".");
        path.push("system32");
        let path_str = super::path_to_string(&path);
        assert_eq!(path_str, "windows/system32");
    }
}
