use std::{ffi::OsStr, path::Path};

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
}

impl Default for FileVisibilityPolicy {
    fn default() -> Self {
        Self {
            read_ignore: false,
            read_hidden: true,
            read_git_ignore: false,
            read_git_exclude: false,
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

    /// Walks through a directory using [`ignore::Walk`]
    pub fn build_walker(&self, path: impl AsRef<Path>) -> ignore::Walk {
        let mut builder = ignore::WalkBuilder::new(path);

        builder
            .git_exclude(self.read_git_exclude)
            .git_ignore(self.read_git_ignore)
            .ignore(self.read_ignore)
            .hidden(self.read_hidden);

        if self.read_git_ignore {
            builder.filter_entry(|p| p.path().file_name() != Some(OsStr::new(".git")));
        }

        builder.build()
    }
}
