//! Implementation of the 'list' command, print list of files in an archive

use std::path::PathBuf;

/// Options controlling how archive contents should be listed
#[derive(Debug, Clone, Copy)]
pub struct ListOptions {
    /// Whether to show a tree view
    pub tree: bool,
}

/// Represents a single file in an archive, used in `list::list_files()`
#[derive(Debug, Clone)]
pub struct FileInArchive {
    /// The file path
    pub path: PathBuf,
}

/// Actually print the files
pub fn list_files(files: Vec<FileInArchive>, list_options: ListOptions) {
    if list_options.tree {
        todo!("Implement tree view");
    } else {
        for file in files {
            println!("{}", file.path.display());
        }
    }
}
