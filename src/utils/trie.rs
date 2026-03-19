use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use indexmap::IndexMap;

pub struct PathTrie {
    is_path: bool,
    children: IndexMap<OsString, PathTrie, rapidhash::fast::RandomState>,
    node_renaming: Option<PathBuf>,
}

impl PathTrie {
    pub fn new() -> Self {
        Self {
            is_path: false,
            children: IndexMap::default(),
            node_renaming: None,
        }
    }

    pub fn contains_ancestor_of(&self, path: &Path) -> bool {
        let (first, rest) = path_split_first(path);

        if rest.as_os_str().is_empty() {
            return false;
        }

        let Some(first) = first else {
            return false;
        };

        let Some(node) = self.children.get(first) else {
            return false;
        };

        if node.is_path {
            return true;
        }

        node.contains_ancestor_of(rest)
    }

    /// Inserts a path into this Trie.
    ///
    /// # Panics:
    ///
    /// - Panics if `path` isn't absolute.
    pub fn insert(&mut self, path: &Path) {
        debug_assert!(path.is_absolute(), "PathTrie only accepts absolute paths");
        self.insert_recursive(path);
    }

    fn insert_recursive(&mut self, path: &Path) {
        let path = path.to_owned();
        let (first, rest) = path_split_first(&path);

        if let Some(first) = first {
            let node = self.children.entry(first.to_owned()).or_insert_with(PathTrie::new);

            if rest.iter().next().is_none() {
                node.is_path = true;
            }
            node.insert_recursive(rest);
        }
    }
}

impl<Item> FromIterator<Item> for PathTrie
where
    Item: AsRef<Path>,
{
    fn from_iter<T: IntoIterator<Item = Item>>(iter: T) -> Self {
        let mut trie = PathTrie::new();
        for path in iter {
            trie.insert(path.as_ref());
        }
        trie
    }
}

fn path_split_first(path: &Path) -> (Option<&OsStr>, &Path) {
    let mut iter = path.iter();
    (iter.next(), iter.as_path())
}
