use std::{
    fs,
    io::{self, BufWriter, Write},
    path::Path,
};

use backhand::{FilesystemReader, InnerNode, SquashfsFileReader};
use filetime_creation::{set_file_handle_times, set_file_mtime, FileTime};

use crate::{
    list::FileInArchive,
    utils::{
        logger::{info, warning},
        Bytes,
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
                    std::os::unix::fs::symlink(&target, &file_path)?;
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
