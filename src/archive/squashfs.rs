use std::{
    env, fs,
    io::{self, BufWriter, Seek, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use backhand::{
    compression::Compressor, FilesystemCompressor, FilesystemReader, FilesystemWriter, InnerNode, NodeHeader,
    SquashfsFileReader,
};
use filetime_creation::{set_file_handle_times, FileTime};
use same_file::Handle;

use crate::{
    error::FinalError,
    list::FileInArchive,
    utils::{
        logger::{info, warning},
        Bytes, FileVisibilityPolicy,
    },
};

pub fn list_archive<'a>(archive: FilesystemReader<'a>) -> impl Iterator<Item = crate::Result<FileInArchive>> + 'a {
    archive.root.nodes.into_iter().filter_map(move |f| {
        // The reported paths are absolute, and include the root directory `/`.
        // To be consistent with outputs of other formats, we strip the prefix `/` and ignore the root directory.
        if f.fullpath == Path::new("/") {
            return None;
        }
        Some(Ok(FileInArchive {
            is_dir: matches!(f.inner, InnerNode::Dir(_)),
            path: f
                .fullpath
                .strip_prefix("/")
                .expect("paths must be absolute")
                .to_path_buf(),
        }))
    })
}

pub fn unpack_archive(archive: FilesystemReader<'_>, output_folder: &Path, quiet: bool) -> crate::Result<usize> {
    let mut unpacked_files = 0usize;

    for f in archive.files() {
        // `output_folder` should already be created.
        if f.fullpath == Path::new("/") {
            continue;
        }

        let relative_path = f.fullpath.strip_prefix("/").expect("paths must be absolute");
        let file_path = output_folder.join(relative_path);

        let mtime = FileTime::from_unix_time(f.header.mtime.into(), 0);

        let warn_ignored = |inode_type: &str| {
            warning(format!("ignored {inode_type} in archive {relative_path:?}"));
        };

        match &f.inner {
            InnerNode::Dir(_) => {
                if !quiet {
                    info(format!("extracting directory {file_path:?}"));
                }
                fs::create_dir(&file_path)?;
                // Directory mtime is not recovered. It will be overwritten by
                // the creation of inner files. We would need a second pass to do so.
            }
            InnerNode::File(file) => {
                if !quiet {
                    let file_size = Bytes::new(match file {
                        SquashfsFileReader::Basic(f) => f.file_size.into(),
                        SquashfsFileReader::Extended(f) => f.file_size,
                    });
                    info(format!("extracting file ({file_size}) {file_path:?}"));
                }

                let mut reader = archive.file(file).reader();
                let output_file = fs::File::create(&file_path)?;
                let mut output_file = BufWriter::new(output_file);
                io::copy(&mut reader, &mut output_file)?;
                output_file.flush()?;
                set_file_handle_times(output_file.get_ref(), None, Some(mtime), None)?;
            }
            InnerNode::Symlink(symlink) => {
                if !quiet {
                    info(format!("extracting symlink {file_path:?}"));
                }

                let target = &symlink.link;
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(target, &file_path)?;
                    filetime_creation::set_symlink_file_times(&file_path, mtime, mtime, mtime)?;
                    // Note: Symlink permissions are ignored on *NIX anyway. No need to set them.
                }

                #[cfg(windows)]
                std::os::windows::fs::symlink_file(&target, &file_path)?;

                // Symlink mtime is specially handled above. Skip the normal handler.
                unpacked_files += 1;
                continue;
            }

            // TODO: Named pipes and sockets *CAN* be created by unprivileged users.
            // Should we extract them by default?
            InnerNode::NamedPipe => {
                warn_ignored("named pipe");
                continue;
            }
            InnerNode::Socket => {
                warn_ignored("socket");
                continue;
            }

            // Not possible without root permission.
            InnerNode::CharacterDevice(_) => {
                warn_ignored("character device");
                continue;
            }
            InnerNode::BlockDevice(_) => {
                warn_ignored("block device");
                continue;
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&file_path, fs::Permissions::from_mode(f.header.permissions.into()))?;
        }

        unpacked_files += 1;
    }

    Ok(unpacked_files)
}

// Re-assignments work bettwe with `cfg` blocks.
#[allow(clippy::field_reassign_with_default)]
pub fn build_archive_from_paths<W>(
    input_filenames: &[PathBuf],
    output_path: &Path,
    mut writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    quiet: bool,
    follow_symlinks: bool,
) -> crate::Result<W>
where
    W: Write + Seek,
{
    let root_dir = match input_filenames {
        [path] if path.is_dir() => path,
        _ => {
            let error = FinalError::with_title("Cannot build squashfs")
                .detail("Squashfs requires a single directory input for root directory")
                .detail(if input_filenames.len() != 1 {
                    "Multiple paths are provided".into()
                } else {
                    format!("Not a directory: {:?}", input_filenames[0])
                });
            return Err(error.into());
        }
    };

    let output_handle = Handle::from_path(output_path);

    let mut fs_writer = FilesystemWriter::default();
    // Set the default compression to Gzip with default level, matching mksquashfs's default.
    // The default choice of `backhand` is Xz which is not enabled by us.
    // TODO: We do not support customization argument for archive formats.
    fs_writer.set_compressor(FilesystemCompressor::new(Compressor::Gzip, None).expect("gzip is supported"));

    // cd *into* the source directory, using it as the archive root.
    let previous_cwd = env::current_dir()?;
    env::set_current_dir(root_dir)?;

    for entry in file_visibility_policy.build_walker(".") {
        let entry = entry?;
        let path = entry.path();

        if let Ok(handle) = &output_handle {
            if matches!(Handle::from_path(path), Ok(x) if &x == handle) {
                warning(format!(
                    "Cannot compress `{}` into itself, skipping",
                    output_path.display()
                ));
            }
        }

        if !quiet {
            // `fs_writer.push_*` only maintains metadata. We do not want to give
            // users a false information that we are compressing during the
            // traversal. So this is not "compressing".
            // File reading, compression and writing are done in
            // `fs_writer.write` below, after the hierarchy tree gets finalized.
            info(format!("Found {path:?}"));
        }

        let metadata = entry.metadata()?;
        let file_type = metadata.file_type();

        let mut header = NodeHeader::default();
        header.mtime = match metadata.modified() {
            // Not available.
            Err(_) => 0,
            Ok(mtime) => {
                let mtime = mtime
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .and_then(|dur| u32::try_from(dur.as_secs()).ok());
                if mtime.is_none() {
                    warning(format!(
                        "Modification time of {path:?} exceeds the representable range (1970-01-01 ~ 2106-02-07) \
                        of squashfs. Recorded as 1970-01-01."
                    ));
                }
                mtime.unwrap_or(0)
            }
        };

        #[cfg(not(unix))]
        {
            header.permissions = 0o777;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            // Only permission bits, not file type bits.
            header.permissions = metadata.permissions().mode() as u16 & !(libc::S_IFMT as u16);
            header.uid = metadata.uid();
            header.gid = metadata.gid();
        }

        // Root directory is special cased.
        if path == Path::new(".") {
            fs_writer.set_root_mode(header.permissions);
            fs_writer.set_root_uid(header.uid);
            fs_writer.set_root_gid(header.gid);
            continue;
        }

        if file_type.is_dir() {
            fs_writer.push_dir(path, header)?;
        } else if !follow_symlinks && file_type.is_symlink() {
            let target = fs::read_link(path)?;
            fs_writer.push_symlink(target, path, header)?;
        } else if maybe_push_unix_special_inode(&mut fs_writer, path, &metadata, header)? {
            // Already handled.
        } else {
            // Fallback case: read as a regular file.
            // See comments of `LazyFile` for why not `File::open` here.
            let reader = LazyFile::Path(path.to_path_buf());
            fs_writer.push_file(reader, path, header)?;
        }
    }

    if !quiet {
        info("Compressing data".to_string());
    }

    // Finalize the superblock and write data. This should be done before
    // resetting current directory, because `LazyFile`s store relative paths.
    fs_writer.write(&mut writer)?;

    env::set_current_dir(previous_cwd)?;

    Ok(writer)
}

#[cfg(not(unix))]
fn maybe_push_unix_special_inode(
    _writer: &mut FilesystemWriter,
    _path: &Path,
    _metadata: &fs::Metadata,
    _header: NodeHeader,
) -> io::Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn maybe_push_unix_special_inode(
    writer: &mut FilesystemWriter,
    path: &Path,
    metadata: &fs::Metadata,
    header: NodeHeader,
) -> io::Result<bool> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let file_type = metadata.file_type();
    if file_type.is_fifo() {
        writer.push_fifo(path, header)?;
    } else if file_type.is_socket() {
        writer.push_socket(path, header)?;
    } else if file_type.is_block_device() {
        let dev = metadata.rdev() as u32;
        writer.push_block_device(dev, path, header)?;
    } else if file_type.is_char_device() {
        let dev = metadata.rdev() as u32;
        writer.push_char_device(dev, path, header)?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

/// Delay file opening until the first read and close it as soon as the EOF is encountered.
///
/// Due to design of `backhand`, we need to store all `impl Read` into the
/// builder during traversal and write out the squashfs later. But we cannot
/// open and store all file handles during traversal or it will exhaust file
/// descriptors on *NIX if there are thousands of files (a pretty low limit!).
///
/// Upstream discussion: https://github.com/wcampbell0x2a/backhand/discussions/614
enum LazyFile {
    Path(PathBuf),
    Opened(fs::File),
    Closed,
}

impl io::Read for LazyFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            LazyFile::Path(path) => {
                let file = fs::File::open(path)?;
                *self = Self::Opened(file);
                self.read(buf)
            }
            LazyFile::Opened(file) => {
                let cnt = file.read(buf)?;
                if !buf.is_empty() && cnt == 0 {
                    *self = Self::Closed;
                }
                Ok(cnt)
            }
            LazyFile::Closed => Ok(0),
        }
    }
}
