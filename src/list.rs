//! Implementation of the 'list' command, print list of files in an archive

use self::tree::Tree;
use std::path::{Path, PathBuf};

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
pub fn list_files(archive: &Path, files: Vec<FileInArchive>, list_options: ListOptions) {
    println!("{}:", archive.display());
    if list_options.tree {
        let tree: Tree = files.into_iter().collect();
        tree.print();
    } else {
        for FileInArchive { path, is_dir } in files {
            print_entry(path.display(), is_dir);
        }
    }
}

fn print_entry(name: impl std::fmt::Display, is_dir: bool) {
    use crate::utils::colors::*;

    if is_dir {
        // if colors are deactivated, print final / to mark directories
        if BLUE.is_empty() {
            println!("{}/", name);
        } else {
            println!("{}{}{}{}", *BLUE, *STYLE_BOLD, name, *ALL_RESET);
        }
    } else {
        // not a dir -> just print the file name
        println!("{}", name);
    }
}

mod tree {
    use super::FileInArchive;
    use linked_hash_map::LinkedHashMap;
    use std::ffi::OsString;
    use std::iter::FromIterator;
    use std::path;

    const TREE_PREFIX_EMPTY: &str = "   ";
    const TREE_PREFIX_LINE: &str = "│  ";
    const TREE_FINAL_BRANCH: &str = "├── ";
    const TREE_FINAL_LAST: &str = "└── ";

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
                        eprintln!(
                            "[warning] multiple files with the same name in a single directory ({})",
                            file.path.display()
                        )
                    }
                }
            }
        }

        /// Print the file tree using Unicode line characters
        pub fn print(&self) {
            for (i, (name, subtree)) in self.children.iter().enumerate() {
                subtree.print_(name, String::new(), i == self.children.len() - 1);
            }
        }
        /// Print the tree by traversing it recursively
        fn print_(&self, name: &OsString, mut prefix: String, last: bool) {
            // Convert `name` to valid unicode
            let name = name.to_string_lossy();

            // If there are no further elements in the parent directory, add
            // "└── " to the prefix, otherwise add "├── "
            let final_part = match last {
                true => TREE_FINAL_LAST,
                false => TREE_FINAL_BRANCH,
            };

            print!("{}{}", prefix, final_part);
            let is_dir = match self.file {
                Some(FileInArchive { is_dir, .. }) => is_dir,
                None => true,
            };
            super::print_file(name, is_dir);

            // Construct prefix for children, adding either a line if this isn't
            // the last entry in the parent dir or empty space if it is.
            prefix.push_str(match last {
                true => TREE_PREFIX_EMPTY,
                false => TREE_PREFIX_LINE,
            });
            // Recursively print all children
            for (i, (name, subtree)) in self.children.iter().enumerate() {
                subtree.print_(name, prefix.clone(), i == self.children.len() - 1);
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
}
