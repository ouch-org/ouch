//! Some implementation helpers related to the 'list' command.

use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
};

use self::tree::Tree;
use crate::accessible::is_running_in_accessible_mode;

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

    /// Whether this file is a directory
    pub is_dir: bool,
}

/// Actually print the files
/// Returns an Error, if one of the files can't be read
pub fn list_files(
    archive: &Path,
    files: impl IntoIterator<Item = crate::Result<FileInArchive>>,
    list_options: ListOptions,
) -> crate::Result<()> {
    let out = &mut stdout().lock();
    let _ = writeln!(out, "Archive: {}", archive.display());

    if list_options.tree {
        let tree = files.into_iter().collect::<crate::Result<Tree>>()?;
        tree.print(out);
    } else {
        for file in files {
            let FileInArchive { path, is_dir } = file?;
            print_entry(out, &path.display(), is_dir);
        }
    }
    Ok(())
}

/// Print an entry and highlight directories, either by coloring them
/// if that's supported or by adding a trailing /
fn print_entry(out: &mut impl Write, name: impl std::fmt::Display, is_dir: bool) {
    use crate::utils::colors::*;

    if is_dir {
        // if colors are deactivated, print final / to mark directories
        if BLUE.is_empty() {
            let _ = writeln!(out, "{name}/");
        // if in ACCESSIBLE mode, use colors but print final / in case colors
        // aren't read out aloud with a screen reader or aren't printed on a
        // braille reader
        } else if is_running_in_accessible_mode() {
            let _ = writeln!(out, "{}{}{}/{}", *BLUE, *STYLE_BOLD, name, *ALL_RESET);
        } else {
            let _ = writeln!(out, "{}{}{}{}", *BLUE, *STYLE_BOLD, name, *ALL_RESET);
        }
    } else {
        // not a dir -> just print the file name
        let _ = writeln!(out, "{name}");
    }
}

/// Since archives store files as a list of entries -> without direct
/// directory structure (the directories are however part of the name),
/// we have to construct the tree structure ourselves to be able to
/// display them as a tree
mod tree {
    use std::{
        ffi::{OsStr, OsString},
        io::Write,
        path,
    };

    use bstr::{ByteSlice, ByteVec};
    use linked_hash_map::LinkedHashMap;

    use super::FileInArchive;
    use crate::warning;

    /// Directory tree
    #[derive(Debug, Default)]
    pub struct Tree {
        file: Option<FileInArchive>,
        children: LinkedHashMap<OsString, Tree>,
    }

    impl Tree {
        /// Insert a file into the tree
        pub fn insert(&mut self, file: FileInArchive) {
            self.insert_(file.clone(), file.path.iter());
        }
        /// Insert file by traversing the tree recursively
        fn insert_(&mut self, file: FileInArchive, mut path: path::Iter) {
            // Are there more components in the path? -> traverse tree further
            if let Some(part) = path.next() {
                // Either insert into an existing child node or create a new one
                if let Some(t) = self.children.get_mut(part) {
                    t.insert_(file, path)
                } else {
                    let mut child = Tree::default();
                    child.insert_(file, path);
                    self.children.insert(part.to_os_string(), child);
                }
            } else {
                // `path` was empty -> we reached our destination and can insert
                // `file`, assuming there is no file already there (which meant
                // there were 2 files with the same name in the same directory
                // which should be impossible in any sane file system)
                match &self.file {
                    None => self.file = Some(file),
                    Some(file) => {
                        warning!(
                            "multiple files with the same name in a single directory ({})",
                            &file.path.display(),
                        );
                    }
                }
            }
        }

        /// Print the file tree using Unicode line characters
        pub fn print(&self, out: &mut impl Write) {
            for (i, (name, subtree)) in self.children.iter().enumerate() {
                subtree.print_(out, name, "", i == self.children.len() - 1);
            }
        }
        /// Print the tree by traversing it recursively
        fn print_(&self, out: &mut impl Write, name: &OsStr, prefix: &str, last: bool) {
            // If there are no further elements in the parent directory, add
            // "└── " to the prefix, otherwise add "├── "
            let final_part = match last {
                true => draw::FINAL_LAST,
                false => draw::FINAL_BRANCH,
            };

            print!("{prefix}{final_part}");
            let is_dir = match self.file {
                Some(FileInArchive { is_dir, .. }) => is_dir,
                None => true,
            };
            super::print_entry(out, <Vec<u8> as ByteVec>::from_os_str_lossy(name).as_bstr(), is_dir);

            // Construct prefix for children, adding either a line if this isn't
            // the last entry in the parent dir or empty space if it is.
            let mut prefix = prefix.to_owned();
            prefix.push_str(match last {
                true => draw::PREFIX_EMPTY,
                false => draw::PREFIX_LINE,
            });
            // Recursively print all children
            for (i, (name, subtree)) in self.children.iter().enumerate() {
                subtree.print_(out, name, &prefix, i == self.children.len() - 1);
            }
        }
    }

    impl FromIterator<FileInArchive> for Tree {
        fn from_iter<I: IntoIterator<Item = FileInArchive>>(iter: I) -> Self {
            let mut tree = Self::default();
            for file in iter {
                tree.insert(file);
            }
            tree
        }
    }

    /// Constants containing the visual parts of which the displayed tree
    /// is constructed.
    ///
    /// They fall into 2 categories: the `PREFIX_*` parts form the first
    /// `depth - 1` parts while the `FINAL_*` parts form the last part,
    /// right before the entry itself
    ///
    /// `PREFIX_EMPTY`: the corresponding dir is the last entry in its parent dir
    /// `PREFIX_LINE`: there are other entries after the corresponding dir
    /// `FINAL_LAST`: this entry is the last entry in its parent dir
    /// `FINAL_BRANCH`: there are other entries after this entry
    mod draw {
        /// the corresponding dir is the last entry in its parent dir
        pub const PREFIX_EMPTY: &str = "   ";
        /// there are other entries after the corresponding dir
        pub const PREFIX_LINE: &str = "│  ";
        /// this entry is the last entry in its parent dir
        pub const FINAL_LAST: &str = "└── ";
        /// there are other entries after this entry
        pub const FINAL_BRANCH: &str = "├── ";
    }
}
