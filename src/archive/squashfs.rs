use std::path::Path;

use backhand::{FilesystemReader, InnerNode};

use crate::list::FileInArchive;

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
