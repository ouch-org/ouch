//! Some implementation helpers related to the 'list' command.

use std::{
    fmt,
    io::{BufWriter, Write, stdout},
    path::{Path, PathBuf},
};

use self::tree::Tree;
use crate::{Result, accessible::is_running_in_accessible_mode, utils::PathFmt};

/// Options controlling how archive contents should be listed
#[derive(Debug, Clone, Copy)]
pub struct ListOptions {
    /// Whether to show a tree view
    pub tree: bool,

    /// Whether to suppress extra output like symlink targets (for scripting)
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListFileType {
    File,
    Directory,
    Symlink { target: PathBuf },
    Hardlink { target: PathBuf },
}

/// Represents a single file in an archive, used in `list::list_files()`
#[derive(Debug, Clone)]
pub struct FileInArchive {
    /// The file path
    pub path: PathBuf,

    /// The type of file
    pub file_type: ListFileType,
}

/// Actually print the files
/// Returns an Error, if one of the files can't be read
pub fn list_files(
    archive: &Path,
    files: impl IntoIterator<Item = Result<FileInArchive>>,
    list_options: ListOptions,
) -> Result<()> {
    let mut out = BufWriter::new(stdout().lock());

    if !list_options.quiet {
        let _ = writeln!(out, "Archive: {}", PathFmt(archive));
    }

    if list_options.tree {
        let tree = files.into_iter().collect::<Result<Tree>>()?;
        tree.print(&mut out);
    } else {
        for file in files {
            let FileInArchive { path, file_type } = file?;
            print_entry(&mut out, path.display(), &file_type, list_options.quiet);
        }
    }
    Ok(())
}

/// Print an entry and highlight directories, either by coloring them
/// if that's supported or by adding a trailing /
fn print_entry(out: &mut impl Write, name: impl fmt::Display, file_type: &ListFileType, quiet: bool) {
    use crate::utils::colors::*;

    match file_type {
        ListFileType::File => {
            let _ = writeln!(out, "{name}");
        }
        ListFileType::Symlink { target } | ListFileType::Hardlink { target } => {
            if quiet {
                // In quiet mode, just print the name (like a regular file)
                // This allows scripts to process the list without parsing arrows
                let _ = writeln!(out, "{}{name}{}", *CYAN, *ALL_RESET);
                return;
            }

            let suffix = if matches!(file_type, ListFileType::Hardlink { .. }) {
                " (hardlink)"
            } else {
                ""
            };

            if is_running_in_accessible_mode() {
                let _ = writeln!(out, "{name} -> {}{suffix}", target.display());
            } else {
                let _ = writeln!(
                    out,
                    "{c}{name}{r} {c}-> {c}{target}{suffix}{r}",
                    c = *CYAN,
                    r = *ALL_RESET,
                    target = target.display()
                );
            }
        }
        ListFileType::Directory => {
            let name_str = name.to_string();
            let display_name = name_str.strip_suffix('/').unwrap_or(&name_str);

            let output = if *DISABLE_COLORED_TEXT || is_running_in_accessible_mode() {
                // Don't use colors and print trailing slash to mark directories
                format!("{display_name}/")
            } else {
                // Normal mode: use colors without trailing slash
                format!("{}{}{}{}", *BLUE, *STYLE_BOLD, display_name, *ALL_RESET)
            };

            let _ = writeln!(out, "{output}");
        }
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
    use indexmap::IndexMap;

    use super::{FileInArchive, ListFileType};
    use crate::{utils::NoQuotePathFmt, warning};

    /// Directory tree
    #[derive(Debug, Default)]
    pub struct Tree {
        file: Option<FileInArchive>,
        children: IndexMap<OsString, Tree>,
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
                    let mut child = Self::default();
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
                            NoQuotePathFmt(&file.path),
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

            let _ = write!(out, "{prefix}{final_part}");
            let file_type = match &self.file {
                Some(FileInArchive { file_type, .. }) => file_type.clone(),
                // If we don't have a file entry but have children, it's an implicit directory
                None => ListFileType::Directory,
            };
            super::print_entry(
                out,
                <Vec<u8> as ByteVec>::from_os_str_lossy(name).as_bstr(),
                &file_type,
                false, // Always show targets in tree view, regardless of --quiet flag
            );

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
