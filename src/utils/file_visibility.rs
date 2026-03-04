use std::{
    ffi::OsStr,
    iter,
    path::{Path, PathBuf},
};

use fs_err as fs;

/// Determines which files should be read or ignored during directory walking
pub struct FileVisibilityPolicy {
    /// Enables reading .ignore files.
    ///
    /// Disabled by default.
    pub read_ignore: bool,

    /// If enabled, ignores hidden files.
    ///
    /// Disabled by default
    pub read_hidden: bool,

    /// Enables reading .gitignore files.
    ///
    /// This is enabled by default.
    pub read_git_ignore: bool,

    /// Enables reading `.git/info/exclude` files.
    pub read_git_exclude: bool,

    pub follow_symlinks: bool,
}

impl Default for FileVisibilityPolicy {
    fn default() -> Self {
        Self {
            read_ignore: false,
            read_hidden: true,
            read_git_ignore: false,
            read_git_exclude: false,
            follow_symlinks: false,
        }
    }
}

impl FileVisibilityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    /// Enables reading .ignore files.
    pub fn read_ignore(self, read_ignore: bool) -> Self {
        Self { read_ignore, ..self }
    }

    #[must_use]
    /// Enables reading .gitignore files.
    pub fn read_git_ignore(self, read_git_ignore: bool) -> Self {
        Self {
            read_git_ignore,
            ..self
        }
    }

    #[must_use]
    /// Enables reading `.git/info/exclude` files.
    pub fn read_git_exclude(self, read_git_exclude: bool) -> Self {
        Self {
            read_git_exclude,
            ..self
        }
    }

    #[must_use]
    /// Enables reading `.git/info/exclude` files.
    pub fn read_hidden(self, read_hidden: bool) -> Self {
        Self { read_hidden, ..self }
    }

    #[must_use]
    pub fn follow_symlinks(self, follow_symlinks: bool) -> Self {
        Self {
            follow_symlinks,
            ..self
        }
    }

    /// Walks through a directory using [`ignore::Walk`]
    pub fn build_walker(&self, path: impl AsRef<Path>) -> ignore::Walk {
        let mut builder = ignore::WalkBuilder::new(path);

        builder
            .git_exclude(self.read_git_exclude)
            .git_ignore(self.read_git_ignore)
            .ignore(self.read_ignore)
            .hidden(self.read_hidden)
            .follow_links(true);

        if self.read_git_ignore {
            builder.filter_entry(|p| p.path().file_name().is_some_and(|name| name != ".git"));
            builder.require_git(false);
        }

        builder.build()
    }

    // workaround for ignore::Walk failing if the first given path is a broken symlink
    // even if follow_symlinks is set to false
    //
    // used by tar and zip
    pub fn workaround_build_walker_or_broken_link_path(
        &self,
        explicit_path: &Path,
        filename: &OsStr,
    ) -> Box<dyn Iterator<Item = Result<PathBuf, ignore::Error>> + 'static> {
        let is_broken_symlink = explicit_path.is_symlink() && fs::metadata(explicit_path).is_err();

        let iter: Box<dyn Iterator<Item = Result<PathBuf, ignore::Error>>> = if is_broken_symlink {
            Box::new(iter::once(Ok(PathBuf::from(filename))))
        } else {
            Box::new(
                self.build_walker(filename)
                    .map(|result| result.map(ignore::DirEntry::into_path)),
            )
        };
        iter
    }
}
