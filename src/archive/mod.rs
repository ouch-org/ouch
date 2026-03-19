#[cfg(feature = "unrar")]
pub mod rar;
pub mod sevenz;
pub mod tar;
pub mod zip;

use std::path::{self, Path, PathBuf};

use fs_err as fs;

use crate::{
    FinalError, Result, info,
    utils::{FileType, PathFmt, PathTrie, UnpackEntryType, option_read_file_type},
};

/// Conflict resolver and interface for unpacking.
pub struct ConflictResolver {
    output_folder: PathBuf,
    renamed_parents: PathTrie,
}

#[derive(Debug)]
enum ConflictResolution {
    Proceed(PathBuf),
    SkipEntry,
}

impl ConflictResolver {
    fn new(output_folder: impl Into<PathBuf>) -> Self {
        Self {
            output_folder: output_folder.into(),
            renamed_parents: PathTrie::new(),
        }
    }

    fn renamed(&mut self, before: &Path, after: &Path) {
        a
    }

    fn resolve_archive_path(&self, relative_path: &Path) -> Result<PathBuf> {
        let joined_path = self.output_folder.join(relative_path);
        self.check_unpack_path_safety(&joined_path)?;
        Ok(joined_path)
    }

    fn resolve_archive_path_and_check_conflicts(
        &self,
        path: &Path,
        entry_type: UnpackEntryType,
    ) -> Result<ConflictResolution> {
        let resolved_path = self.resolve_archive_path(path)?;
        self.check_file_type_conflicts(resolved_path, entry_type)
    }

    fn check_file_type_conflicts(
        &self,
        joined_path: PathBuf,
        entry_type: UnpackEntryType,
    ) -> Result<ConflictResolution> {
        let default = || Ok(ConflictResolution::Proceed(joined_path.to_owned()));

        // If there is a file in that path, then we have a conflict
        let Some(type_in_fs) = option_read_file_type(&joined_path)? else {
            return default();
        };

        match (entry_type, type_in_fs) {
            (_, FileType::Regular) if fs::symlink_metadata(&joined_path)?.len() == 0 => {
                fs::remove_file(&joined_path)?;
                info!("automatically overwriting empty file at {}", PathFmt(&joined_path));
                default()
            }
            (_, FileType::Regular) => {
                todo!("conflict!");
            }
            (UnpackEntryType::Directory, FileType::Directory) => Ok(ConflictResolution::SkipEntry),

            (UnpackEntryType::Regular, FileType::Directory) => todo!(),
            (UnpackEntryType::Regular, FileType::Symlink) => todo!(),
            (UnpackEntryType::Directory, FileType::Symlink) => todo!(),
            (UnpackEntryType::Symlink, FileType::Directory) => todo!(),
            (UnpackEntryType::Symlink, FileType::Symlink) => todo!(),
            (UnpackEntryType::HardLink, FileType::Directory) => todo!(),
            (UnpackEntryType::HardLink, FileType::Symlink) => todo!(),
            (UnpackEntryType::Char, FileType::Directory) => todo!(),
            (UnpackEntryType::Char, FileType::Symlink) => todo!(),
            (UnpackEntryType::Block, FileType::Directory) => todo!(),
            (UnpackEntryType::Block, FileType::Symlink) => todo!(),
            (UnpackEntryType::Fifo, FileType::Directory) => todo!(),
            (UnpackEntryType::Fifo, FileType::Symlink) => todo!(),
        }
    }

    fn check_unpack_path_safety(&self, joined_path: &Path) -> Result<()> {
        // Note: this code assumes the process is in the correct folder, all
        // unpack implementations should cd into the output_folder
        let absolute = path::absolute(joined_path)?;

        if absolute.strip_prefix(&self.output_folder).is_err() {
            return Err(FinalError::with_title("file to unpack failed path safety check")
                .detail(format!("path is outside of {}", PathFmt(&self.output_folder)))
                .detail(format!("path in archive was read as {}", PathFmt(joined_path)))
                .detail(format!("and was resolved to {}", PathFmt(&absolute)))
                .hint("this is prohibited due to safety reasons (see CVE-2001-1267)")
                .into());
        }

        Ok(())
    }

    // pub fn create_symlink(&self, target: &Path, link_location: &PathBuf) -> Result<()> {
    //     utils::create_symlink(&target, &link_location)
    // }

    // pub fn create_hard_link(&self, target: &Path, link_location: &PathBuf) -> Result<()> {
    //     Ok(fs::hard_link(&target, &link_location)?)
    // }
}
