//! Contains Zip-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    borrow::Cow,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};

use encoding_rs::Encoding;
use filetime_creation::{FileTime, set_file_mtime};
use fs_err as fs;
use is_executable::is_executable;
use same_file::Handle;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use typed_path::{Utf8WindowsComponent, Utf8WindowsPath};
use zip::{self, DateTime, HasZipMetadata, ZipArchive, read::ZipFile};

#[cfg(unix)]
use crate::utils::sanitize_archive_mode;
use crate::{
    QuestionPolicy, Result,
    error::FinalError,
    info, info_accessible,
    list::{FileInArchive, ListFileType},
    utils::{
        BytesFmt, FileType, FileVisibilityPolicy, PathFmt, canonicalize, cd_into_same_dir_as,
        copy_limited_decompression, create_symlink, ensure_parent_dir_exists, get_invalid_utf8_paths,
        is_same_file_as_output, pretty_format_list_of_paths, read_file_type, resolve_extraction_conflict,
        strip_cur_dir, validate_dest_inside_root, validate_symlink_target,
    },
    warning,
};

/// Unpacks the archive given by `archive` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
///
/// `name_encoding` is the optional charset used to decode entry names whose UTF-8
/// flag is unset (see `--encoding`).
pub fn unpack_archive<R>(
    reader: R,
    output_folder: &Path,
    password: Option<&[u8]>,
    name_encoding: Option<&'static Encoding>,
    question_policy: QuestionPolicy,
) -> Result<u64>
where
    R: Read + Seek,
{
    let mut files_unpacked = 0;
    let mut archive = ZipArchive::new(reader)?;

    for idx in 0..archive.len() {
        let mut file = match password {
            Some(password) => archive.by_index_decrypt(idx, password)?,
            None => archive.by_index(idx)?,
        };
        let entry_name = decoded_entry_name(&file, name_encoding);
        let relpath = match enclosed_name(&entry_name) {
            Some(path) => path,
            None => {
                warning!("skipping entry {} with unsafe name: {}", idx, entry_name);
                continue;
            }
        };

        let file_path = output_folder.join(&relpath);

        validate_dest_inside_root(output_folder, &file_path)?;

        display_zip_comment_if_exists(&file, &entry_name);

        match file.is_dir() {
            _is_dir @ true => {
                info!("Directory {} created", PathFmt(&file_path));

                let mode = file.unix_mode();
                let is_symlink = mode.is_some_and(|mode| mode & 0o170000 == 0o120000);

                if is_symlink {
                    // Symlink targets are arbitrary bytes on Unix, not guaranteed UTF-8; read as bytes.
                    let mut target_bytes = Vec::new();
                    file.read_to_end(&mut target_bytes)?;
                    let target = symlink_target_from_bytes(&target_bytes);

                    validate_symlink_target(&relpath, &target)?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&target, &file_path)?;
                    #[cfg(windows)]
                    std::os::windows::fs::symlink_dir(&target, file_path)?;
                } else {
                    fs::create_dir_all(&file_path)?;
                }
            }
            _is_file @ false => {
                ensure_parent_dir_exists(&file_path)?;
                let file_path = strip_cur_dir(file_path.as_path());

                let mode = file.unix_mode();
                let is_symlink = mode.is_some_and(|mode| mode & 0o170000 == 0o120000);

                // Symlink creation fails on its own when the path is taken.
                let mut resolved = None;
                if !is_symlink {
                    let Some(path) = resolve_extraction_conflict(file_path, question_policy)? else {
                        continue;
                    };
                    resolved = Some(path);
                }
                let file_path = resolved.as_deref().unwrap_or(file_path);

                if is_symlink {
                    // Symlink targets are arbitrary bytes on Unix, not guaranteed UTF-8; read as bytes.
                    let mut target_bytes = Vec::new();
                    file.read_to_end(&mut target_bytes)?;
                    let target = symlink_target_from_bytes(&target_bytes);

                    validate_symlink_target(&relpath, &target)?;
                    info!("linking {} -> \"{}\"", PathFmt(file_path), target.display());

                    create_symlink(&target, file_path)?;
                } else {
                    #[cfg(unix)]
                    let mut output_file = {
                        use fs_err::os::unix::fs::OpenOptionsExt;
                        let mode = file.unix_mode().and_then(valid_unix_permissions).unwrap_or(0o644);
                        fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .mode(mode)
                            .open(file_path)?
                    };
                    #[cfg(not(unix))]
                    let mut output_file = fs::File::create(file_path)?;
                    {
                        copy_limited_decompression(&mut file, &mut output_file)?;
                    }
                    set_last_modified_time(&file, file_path)?;
                    #[cfg(unix)]
                    unix_set_permissions(file_path, &file)?;
                }

                // same reason is in _is_dir: long, often not needed text
                info!("extracted ({}) {}", BytesFmt(file.size()), PathFmt(file_path));
            }
        }

        files_unpacked += 1;
    }

    Ok(files_unpacked)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive<R>(
    mut archive: ZipArchive<R>,
    password: Option<&[u8]>,
    name_encoding: Option<&'static Encoding>,
) -> impl Iterator<Item = Result<FileInArchive>>
where
    R: Read + Seek,
{
    let password = password.map(|p| p.to_owned());

    (0..archive.len()).map(move |idx| {
        let zip_result = match password.clone() {
            Some(password) => archive.by_index_decrypt(idx, &password),
            None => archive.by_index(idx),
        };

        let mut file = match zip_result {
            Ok(f) => f,
            Err(e) => return Err(e.into()),
        };

        let path = {
            let entry_name = decoded_entry_name(&file, name_encoding);
            enclosed_name(&entry_name).unwrap_or_else(|| mangled_name(&entry_name))
        };
        let size = Some(file.size());

        let file_type = if file.is_dir() {
            ListFileType::Directory
        } else if let Some(target) = file
            .unix_mode()
            .filter(|mode| mode & 0o170000 == 0o120000)
            .and_then(|_| {
                let mut s = Vec::new();
                file.read_to_end(&mut s)
                    .ok()
                    .map(|_| PathBuf::from(String::from_utf8_lossy(&s).into_owned()))
            })
        {
            ListFileType::Symlink { target }
        } else {
            ListFileType::File
        };

        Ok(FileInArchive { path, file_type, size })
    })
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive<W>(
    input_filenames: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    follow_symlinks: bool,
) -> Result<W>
where
    W: Write + Seek,
{
    let mut writer = zip::ZipWriter::new(writer);
    let output_handle = Handle::from_path(output_path);

    // always use ZIP64 to allow compression of files larger than 4GB
    // the format is widely supported and the 20B cost is negligible
    let default_options = zip::write::SimpleFileOptions::default().large_file(true);
    let default_executable_options = default_options.unix_permissions(0o755);

    // Vec of any filename that failed the UTF-8 check
    let invalid_unicode_filenames = get_invalid_utf8_paths(input_filenames);

    if !invalid_unicode_filenames.is_empty() {
        let error = FinalError::with_title("Cannot build zip archive")
            .detail("Zip archives require files to have valid UTF-8 paths")
            .detail(format!(
                "Files with invalid paths: {}",
                pretty_format_list_of_paths(&invalid_unicode_filenames),
            ));

        return Err(error.into());
    }

    for explicit_path in input_filenames {
        let previous_location = cd_into_same_dir_as(explicit_path)?;
        let _cwd_guard = crate::utils::CwdGuard::new(previous_location);

        // Unwrap safety:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = explicit_path.file_name().unwrap();

        let iter = file_visibility_policy.workaround_build_walker_or_broken_link_path(explicit_path, filename);

        for entry in iter {
            let path = entry?;

            // Avoid compressing the output file into itself
            if let Ok(handle) = output_handle.as_ref()
                && is_same_file_as_output(&path, handle)
            {
                warning!("Cannot compress {} into itself, skipping", PathFmt(output_path));
                continue;
            }

            info!("Compressing {}", PathFmt(&path));

            let (metadata, file_type) = {
                if follow_symlinks {
                    (path.metadata()?, read_file_type(canonicalize(&path)?)?)
                } else {
                    (path.symlink_metadata()?, read_file_type(&path)?)
                }
            };

            #[cfg(unix)]
            let mode = metadata.permissions().mode();

            let entry_name = path.to_str().ok_or_else(zip_non_utf8_error(&path))?;
            // ZIP format requires forward slashes as path separators, regardless of platform
            let entry_name = entry_name.replace(std::path::MAIN_SEPARATOR, "/");

            match file_type {
                FileType::Regular => {
                    let options = if cfg!(not(unix)) && is_executable(&path) {
                        default_executable_options
                    } else {
                        default_options
                    };

                    let mut file = fs::File::open(&path)?;

                    #[cfg(unix)]
                    let options = options.unix_permissions(mode);
                    // Updated last modified time
                    let last_modified_time = options.last_modified_time(get_last_modified_time(&file));

                    writer.start_file(entry_name, last_modified_time)?;
                    io::copy(&mut file, &mut writer)?;
                }
                FileType::Directory => {
                    // Directory entries have no data and get an invalid size when ZIP64 is forced on
                    writer.add_directory(entry_name, default_options.large_file(false))?;
                }
                FileType::Symlink => {
                    let target_path = path.read_link()?;
                    let target_name = target_path.to_str().ok_or_else(zip_non_utf8_error(&target_path))?;
                    // ZIP format requires forward slashes as path separators, regardless of platform
                    let target_name = target_name.replace(std::path::MAIN_SEPARATOR, "/");

                    // This approach writes the symlink target path as the content of the symlink entry.
                    // We detect symlinks during extraction by checking for the Unix symlink mode (0o120000) in the entry's permissions.
                    #[cfg(unix)]
                    let symlink_options = default_options.unix_permissions(0o120000 | (mode & 0o777));
                    #[cfg(windows)]
                    let symlink_options = default_options.unix_permissions(0o120777);

                    writer.add_symlink(entry_name, target_name, symlink_options)?;
                }
            }
        }
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

/// Decode a zip symlink target's raw bytes into a path (lossless on Unix, lossy elsewhere).
fn symlink_target_from_bytes(bytes: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Decode an entry's file name.
///
/// Names carrying the UTF-8 flag (or fixed up through the Unicode Path extra field)
/// are already decoded by the `zip` crate. For the rest, the raw bytes are decoded
/// with `fallback_encoding` when one is given; otherwise they are taken as UTF-8 if
/// they are valid UTF-8, or decoded as CP437, which is what the ZIP specification
/// mandates when the UTF-8 flag is unset.
fn decoded_entry_name<'a, R: Read + ?Sized>(
    file: &'a ZipFile<'a, R>,
    fallback_encoding: Option<&'static Encoding>,
) -> Cow<'a, str> {
    if file.get_metadata().is_utf8 {
        return Cow::Borrowed(file.name());
    }
    let raw = file.name_raw();
    if let Some(encoding) = fallback_encoding {
        return encoding.decode(raw).0;
    }
    match std::str::from_utf8(raw) {
        Ok(name) => Cow::Borrowed(name),
        Err(_) => Cow::Borrowed(file.name()),
    }
}

/// Equivalent to the `zip` crate's [`ZipFile::enclosed_name`], but evaluated on an
/// already-decoded entry name instead of `file.name()`.
///
/// Returns `None` if the name contains a NUL byte or could escape the destination
/// directory.
fn enclosed_name(file_name: &str) -> Option<PathBuf> {
    if file_name.contains('\0') {
        return None;
    }
    let mut depth = 0usize;
    let mut out_path = PathBuf::new();
    for component in Utf8WindowsPath::new(file_name).components() {
        match component {
            Utf8WindowsComponent::Prefix(_) | Utf8WindowsComponent::RootDir => {
                if depth > 0 {
                    return None;
                }
            }
            Utf8WindowsComponent::ParentDir => {
                depth = depth.checked_sub(1)?;
                out_path.pop();
            }
            Utf8WindowsComponent::Normal(s) => {
                depth += 1;
                out_path.push(s);
            }
            Utf8WindowsComponent::CurDir => (),
        }
    }
    Some(out_path)
}

/// Equivalent to the `zip` crate's [`ZipFile::mangled_name`], but evaluated on an
/// already-decoded entry name.
fn mangled_name(file_name: &str) -> PathBuf {
    let no_null_filename = match file_name.find('\0') {
        Some(index) => &file_name[0..index],
        None => file_name,
    };
    Utf8WindowsPath::new(no_null_filename)
        .components()
        .filter_map(|component| match component {
            Utf8WindowsComponent::Normal(s) => Some(s),
            _ => None,
        })
        .collect()
}

fn display_zip_comment_if_exists<R: Read>(file: &ZipFile<'_, R>, entry_name: &str) {
    let comment = file.comment();
    if !comment.is_empty() {
        // Zip file comments seem to be pretty rare, but if they are used,
        // they may contain important information, so better show them
        //
        // "The .ZIP file format allows for a comment containing up to 65,535 (216−1) bytes
        // of data to occur at the end of the file after the central directory."
        //
        // If there happen to be cases of very long and unnecessary comments in
        // the future, maybe asking the user if he wants to display the comment
        // (informing him of its size) would be sensible for both normal and
        // accessibility mode..
        info_accessible!("Found comment in {}: {}", entry_name, comment);
    }
}

fn get_last_modified_time(file: &fs::File) -> DateTime {
    file.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| {
            let datetime = OffsetDateTime::from(time);
            let offset = UtcOffset::local_offset_at(datetime).unwrap_or(UtcOffset::UTC);
            zip_datetime_from_offset(datetime, offset)
        })
        .unwrap_or_default()
}

fn set_last_modified_time<R: Read>(zip_file: &ZipFile<'_, R>, path: &Path) -> Result<()> {
    let file_time = zip_file
        .last_modified()
        .and_then(|datetime| PrimitiveDateTime::try_from(datetime).ok())
        .map(|pdt| {
            let offset = local_offset_for(pdt).unwrap_or(UtcOffset::UTC);
            file_time_from_local_datetime(pdt, offset)
        });

    if let Some(modification_time) = file_time {
        set_file_mtime(path, modification_time)?;
    }

    Ok(())
}

fn zip_datetime_from_offset(datetime: OffsetDateTime, offset: UtcOffset) -> Option<DateTime> {
    let local = datetime.to_offset(offset);
    DateTime::try_from(PrimitiveDateTime::new(local.date(), local.time())).ok()
}

fn local_offset_for(datetime: PrimitiveDateTime) -> Option<UtcOffset> {
    let mut offset = UtcOffset::local_offset_at(datetime.assume_utc()).ok()?;

    for _ in 0..2 {
        let next = UtcOffset::local_offset_at(datetime.assume_offset(offset)).ok()?;
        if next == offset {
            break;
        }
        offset = next;
    }

    Some(offset)
}

fn file_time_from_local_datetime(datetime: PrimitiveDateTime, offset: UtcOffset) -> FileTime {
    FileTime::from_unix_time(datetime.assume_offset(offset).unix_timestamp(), 0)
}

/// A zip mode without Unix file-type bits isn't a real Unix mode, so its permissions are ignored.
#[cfg(unix)]
fn valid_unix_permissions(mode: u32) -> Option<u32> {
    (mode & 0o170000 != 0).then(|| sanitize_archive_mode(mode))
}

#[cfg(unix)]
fn unix_set_permissions<R: Read>(file_path: &Path, file: &ZipFile<'_, R>) -> Result<()> {
    use std::fs::Permissions;

    // Apply the entry's permissions only when it carries a valid Unix mode.
    if let Some(mode) = file.unix_mode().and_then(valid_unix_permissions) {
        fs::set_permissions(file_path, Permissions::from_mode(mode))?;
    }

    Ok(())
}

fn zip_non_utf8_error<'a>(path: &'a Path) -> impl Fn() -> FinalError + 'a {
    || {
        FinalError::with_title("Zip requires that all paths are valid UTF-8")
            .detail(format!("File {} has a non-UTF-8 path", PathFmt(path)))
    }
}

#[cfg(test)]
mod tests {
    use time::{Date, Month, Time};

    use super::*;

    #[test]
    fn zip_timestamps_round_trip_with_offset() {
        let date = Date::from_calendar_date(2026, Month::February, 7).unwrap();
        let local = PrimitiveDateTime::new(date, Time::from_hms(10, 6, 50).unwrap());
        let offset = UtcOffset::from_hms(-6, 0, 0).unwrap();
        let instant = local.assume_offset(offset);

        let datetime = zip_datetime_from_offset(instant, offset).unwrap();
        assert_eq!(PrimitiveDateTime::try_from(datetime).unwrap(), local);

        let file_time = file_time_from_local_datetime(local, offset);
        assert_eq!(file_time.unix_seconds(), instant.unix_timestamp());
    }

    #[test]
    fn zip_timestamps_round_trip_with_local_offset() {
        let instant = Date::from_calendar_date(2026, Month::February, 7)
            .unwrap()
            .with_hms(12, 34, 56)
            .unwrap()
            .assume_utc();
        let offset = UtcOffset::local_offset_at(instant).unwrap();
        let datetime = zip_datetime_from_offset(instant, offset).unwrap();
        let local = PrimitiveDateTime::try_from(datetime).unwrap();
        let resolved_offset = local_offset_for(local).unwrap();
        let file_time = file_time_from_local_datetime(local, resolved_offset);
        assert_eq!(file_time.unix_seconds(), instant.unix_timestamp());
    }

    /// Build a minimal stored zip whose entry names are the exact raw bytes given.
    /// `flags` is written to the general purpose bit flag; `0` means "not UTF-8".
    fn build_zip(entries: &[(&[u8], &[u8], u16)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for &(name, data, flags) in entries {
            let crc = crc32fast::hash(data);
            let offset = out.len() as u32;

            // Local file header
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
            out.extend_from_slice(&0u16.to_le_bytes()); // mod time
            out.extend_from_slice(&0u16.to_le_bytes()); // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra field length
            out.extend_from_slice(name);
            out.extend_from_slice(data);

            // Central directory entry
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&flags.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // method: stored
            central.extend_from_slice(&0u16.to_le_bytes()); // mod time
            central.extend_from_slice(&0u16.to_le_bytes()); // mod date
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra field length
            central.extend_from_slice(&0u16.to_le_bytes()); // comment length
            central.extend_from_slice(&0u16.to_le_bytes()); // disk number
            central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
            central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name);
        }

        // End of central directory
        let cd_offset = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number
        out.extend_from_slice(&0u16.to_le_bytes()); // central dir disk
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment length
        out
    }

    /// Extract `zip_bytes` into a fresh tempdir and return every extracted file name.
    fn unpack_and_collect_names(zip_bytes: Vec<u8>, encoding: Option<&'static Encoding>) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        unpack_archive(
            io::Cursor::new(zip_bytes),
            dir.path(),
            None,
            encoding,
            QuestionPolicy::AlwaysYes,
        )
        .unwrap();

        fn names(dir: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    names(&entry.path(), out);
                } else {
                    out.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        let mut out = Vec::new();
        names(dir.path(), &mut out);
        out
    }

    const UTF8_FLAG: u16 = 0x0800;

    /// GBK (CP936) bytes of "这是一个测试文件.txt", as produced by `convmv -t CP936`
    /// followed by `zip` on a system without UTF-8 filenames (issue #691).
    fn gbk_name_bytes() -> Vec<u8> {
        let (encoded, _, had_errors) = Encoding::for_label(b"gbk").unwrap().encode("这是一个测试文件.txt");
        assert!(!had_errors);
        encoded.into_owned()
    }

    #[test]
    fn extracts_non_utf8_name_with_encoding_option() {
        let zip_bytes = build_zip(&[(&gbk_name_bytes(), b"hello", 0)]);
        let names = unpack_and_collect_names(zip_bytes, Encoding::for_label(b"gbk"));
        assert_eq!(names, ["这是一个测试文件.txt"]);
    }

    #[test]
    fn lists_non_utf8_name_with_encoding_option() {
        let zip_bytes = build_zip(&[(&gbk_name_bytes(), b"hello", 0)]);
        let archive = ZipArchive::new(io::Cursor::new(zip_bytes)).unwrap();
        let paths: Vec<PathBuf> = list_archive(archive, None, Encoding::for_label(b"gbk"))
            .map(|entry| entry.unwrap().path)
            .collect();
        assert_eq!(paths, [PathBuf::from("这是一个测试文件.txt")]);
    }

    #[test]
    fn unmarked_utf8_name_is_recovered_without_encoding_option() {
        // Some tools write UTF-8 names but forget to set the UTF-8 flag (issue #691).
        let zip_bytes = build_zip(&[("Schwarz-weiß".as_bytes(), b"hi", 0)]);
        let names = unpack_and_collect_names(zip_bytes, None);
        assert_eq!(names, ["Schwarz-weiß"]);
    }

    #[test]
    fn unmarked_non_utf8_name_defaults_to_cp437() {
        // Without --encoding, non-UTF-8 names keep the spec-mandated CP437 decoding.
        let zip_bytes = build_zip(&[(&gbk_name_bytes(), b"hello", 0)]);
        let mut archive = ZipArchive::new(io::Cursor::new(zip_bytes.clone())).unwrap();
        let cp437_name = archive.by_index(0).unwrap().name().to_owned();

        let names = unpack_and_collect_names(zip_bytes, None);
        assert_eq!(names, [cp437_name]);
    }

    #[test]
    fn utf8_flagged_name_ignores_encoding_option() {
        // Names flagged as UTF-8 always decode as UTF-8, per the ZIP spec.
        let zip_bytes = build_zip(&[("Schwarz-weiß".as_bytes(), b"hi", UTF8_FLAG)]);
        let names = unpack_and_collect_names(zip_bytes, Encoding::for_label(b"gbk"));
        assert_eq!(names, ["Schwarz-weiß"]);
    }

    #[test]
    fn enclosed_name_rejects_unsafe_paths() {
        assert_eq!(enclosed_name("a/b/c.txt"), Some(PathBuf::from("a/b/c.txt")));
        assert_eq!(enclosed_name("../evil.txt"), None);
        assert_eq!(enclosed_name("a/../../evil.txt"), None);
        assert_eq!(enclosed_name("evil\0.txt"), None);
        // Backslashes count as separators, like the `zip` crate's own check.
        assert_eq!(enclosed_name("..\\evil.txt"), None);
    }
}
