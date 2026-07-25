//! Per-process filesystem sandbox.

use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod landlock;

#[derive(Debug, Default, Clone)]
pub struct SandboxPolicy {
    read: Vec<PathBuf>,
    read_write: Vec<PathBuf>,
    /// Directories where the decompressor can delete the input archive but not write nearby.
    remove_in: Vec<PathBuf>,
    /// Directories where only subdirectories may be removed.
    remove_dir_in: Vec<PathBuf>,
    disabled: bool,
}

impl SandboxPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_read<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.read.push(path.into());
        self
    }

    pub fn allow_read_write<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.read_write.push(path.into());
        self
    }

    /// Allow deleting files inside `path` without granting any other write rights.
    pub fn allow_remove_in<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.remove_in.push(path.into());
        self
    }

    /// Allow removing subdirectories of `path` without granting any other write rights.
    pub fn allow_remove_dir_in<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.remove_dir_in.push(path.into());
        self
    }

    pub fn set_disabled(&mut self, disabled: bool) -> &mut Self {
        self.disabled = disabled;
        self
    }

    pub fn read_paths(&self) -> &[PathBuf] {
        &self.read
    }

    pub fn read_write_paths(&self) -> &[PathBuf] {
        &self.read_write
    }

    pub fn remove_in_paths(&self) -> &[PathBuf] {
        &self.remove_in
    }

    pub fn remove_dir_in_paths(&self) -> &[PathBuf] {
        &self.remove_dir_in
    }

    /// Apply the policy and return whether the sandbox is now enforced.
    /// Failures emit a warning and let the process run unrestricted.
    pub fn apply(&self) -> bool {
        if disabled_by_request(self.disabled) {
            return false;
        }
        // On Linux the backend warns instead of failing when Landlock is missing and keeps going
        #[cfg(target_os = "linux")]
        {
            landlock::apply(self)
        }
        // The sandbox is Linux only so elsewhere there is nothing to enforce
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

/// Resolve `path` to an absolute path, falling back to its parent if the
/// target itself doesn't yet exist.
pub fn canonicalize_for_sandbox(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    if let Some(parent) = path.parent()
        && let Ok(c) = std::fs::canonicalize(parent)
    {
        return c.join(path.file_name().unwrap_or_default());
    }
    path.to_path_buf()
}

/// Returns true if `target` is $HOME itself or an ancestor of it.
pub fn is_home_or_ancestor(target: &Path) -> bool {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return false;
    };
    let Ok(home) = std::fs::canonicalize(&home) else {
        return false;
    };
    let target = canonicalize_for_sandbox(target);
    home == target || home.starts_with(&target)
}

/// True when the user turned the sandbox off by flag or by OUCH_NO_SANDBOX.
pub fn disabled_by_request(no_sandbox: bool) -> bool {
    no_sandbox || std::env::var_os("OUCH_NO_SANDBOX").is_some()
}

// whether a sandbox can actually be enforced on this platform
pub fn available() -> bool {
    #[cfg(target_os = "linux")]
    {
        landlock::is_available()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_collects_paths() {
        let mut policy = SandboxPolicy::new();
        policy
            .allow_read("/tmp/in")
            .allow_read_write("/tmp/out")
            .allow_remove_in("/tmp/spill")
            .allow_remove_dir_in("/tmp");

        assert_eq!(policy.read_paths(), &[PathBuf::from("/tmp/in")]);
        assert_eq!(policy.read_write_paths(), &[PathBuf::from("/tmp/out")]);
        assert_eq!(policy.remove_in_paths(), &[PathBuf::from("/tmp/spill")]);
        assert_eq!(policy.remove_dir_in_paths(), &[PathBuf::from("/tmp")]);
    }

    #[test]
    fn policy_disabled_default_false() {
        let policy = SandboxPolicy::new();
        assert!(!policy.disabled);
    }

    #[test]
    fn policy_set_disabled() {
        let mut policy = SandboxPolicy::new();
        policy.set_disabled(true);
        assert!(policy.disabled);
    }

    #[test]
    fn apply_returns_false_when_disabled() {
        let mut policy = SandboxPolicy::new();
        policy.set_disabled(true);
        assert!(!policy.apply());
    }

    #[test]
    fn canonicalize_existing_path_resolves() {
        let dir = tempfile::tempdir().unwrap();
        let canon = canonicalize_for_sandbox(dir.path());
        assert!(canon.is_absolute());
        assert!(canon.exists());
    }

    #[test]
    fn canonicalize_missing_path_falls_back_to_parent() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist-yet.txt");
        let canon = canonicalize_for_sandbox(&missing);
        assert!(canon.is_absolute());
        assert_eq!(canon.file_name().unwrap(), "does-not-exist-yet.txt");
    }
}
