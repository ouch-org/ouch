use std::{borrow::Cow, path::Path};

use crate::CURRENT_DIRECTORY;

/// Converts an OsStr to utf8 with custom formatting.
///
/// This is different from [`Path::display`].
///
/// See <https://gist.github.com/marcospb19/ebce5572be26397cf08bbd0fd3b65ac1> for a comparison.
pub fn to_utf(os_str: &Path) -> Cow<str> {
    let format = || {
        let text = format!("{:?}", os_str);
        Cow::Owned(text.trim_matches('"').to_string())
    };

    os_str.to_str().map_or_else(format, Cow::Borrowed)
}

/// Removes the current dir from the beginning of a path as it's redundant information,
/// useful for presentation sake.
pub fn strip_cur_dir(source_path: &Path) -> &Path {
    let current_dir = &*CURRENT_DIRECTORY;

    source_path.strip_prefix(current_dir).unwrap_or(source_path)
}

/// Converts a slice of AsRef<OsStr> to comma separated String
///
/// Panics if the slice is empty.
pub fn pretty_format_list_of_paths(os_strs: &[impl AsRef<Path>]) -> String {
    let mut iter = os_strs.iter().map(AsRef::as_ref);

    let first_element = iter.next().unwrap();
    let mut string = to_utf(first_element).into_owned();

    for os_str in iter {
        string += ", ";
        string += &to_utf(os_str);
    }
    string
}

/// Display the directory name, but use "current directory" when necessary.
pub fn nice_directory_display(path: &Path) -> Cow<str> {
    if path == Path::new(".") {
        Cow::Borrowed("current directory")
    } else {
        to_utf(path)
    }
}
