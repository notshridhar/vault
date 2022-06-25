use super::ext::{PathExt, VecExt};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const IGNORE: &[&str] = &[".DS_Store"];

/// Lists all files in the given directory.
/// - If the directory does not exist, returns empty list.
/// - Does not match system cache files.
fn list_files<F>(dir: &Path, f: &F) -> Vec<PathBuf>
where F: Fn(&Path) -> Option<PathBuf> {
    if let Ok(read_results) = fs::read_dir(dir) {
        let iter = read_results.filter_map(|entry| {
            let entry_path = entry.unwrap().path();
            let file_name = entry_path.to_filename_str();
            match f(&entry_path) {
                Some(path) => match entry_path.is_file() {
                    true if IGNORE.contains(&file_name) => None,
                    true => Some(path),
                    false => None
                }
                None => None
            }
        });
        iter.collect()
    } else {
        Vec::new()
    }
}

/// Lists all files in the given directory recursively.
/// - If the directory does not exist, returns empty list.
/// - Does not match system cache files.
fn walk_dir<F>(dir: &Path, f: &F) -> Vec<PathBuf>
where F: Fn(&Path) -> Option<PathBuf> {
    if let Ok(read_results) = fs::read_dir(dir) {
        read_results.fold(Vec::with_capacity(4), |accum, item| {
            let entry_path = item.unwrap().path();
            let file_name = entry_path.to_filename_str();
            match f(&entry_path) {
                Some(path) => match entry_path.is_dir() {
                    true => accum.extend_inplace(walk_dir(&entry_path, f)),
                    false if IGNORE.contains(&file_name) => accum,
                    false => accum.push_inplace(path),
                }
                None => accum
            }
        })
    } else {
        Vec::new()
    }
}

/// Removes all empty directories recursively within the given directory.
/// Returns whether the given directory was completely removed.
/// Also removes system cache files in the process.
fn remove_empty_dirs(dir: &Path) -> bool {
    if let Ok(read_results) = fs::read_dir(dir) {
        let is_empty = read_results.fold(true, |accum, item| {
            let entry_path = item.unwrap().path();
            let is_removed = if entry_path.is_dir() {
                remove_empty_dirs(&entry_path)
            } else {
                let file_name = entry_path.to_filename_str();
                match IGNORE.contains(&file_name) {
                    true => fs::remove_file(entry_path).is_ok(),
                    false => false,
                }
            };
            is_removed && accum
        });
        fs::remove_dir(dir).unwrap_or_default();
        is_empty
    } else {
        true
    }
}

#[derive(PartialEq)]
enum PatternKind {
    Exact,
    SameLevel,
    Recursive,
}

pub struct Pattern<'a> {
    value: &'a str,
    kind: PatternKind,
}

impl<'a> Pattern<'a> {
    /// Creates a pattern matcher from its string representation.
    /// - `"pat"` creates `Exact` pattern matcher.
    /// - `"pat*"` creates `SameLevel` pattern matcher.
    /// - `"pat**"` creates `Recursive` pattern matcher.
    pub fn from_str(value: &'a str) -> Self {
        if value.ends_with("**") {
            Self {
                value: value.strip_suffix("**").unwrap(),
                kind: PatternKind::Recursive,
            }
        } else if value.ends_with('*') {
            Self {
                value: value.strip_suffix('*').unwrap(),
                kind: PatternKind::SameLevel,
            }
        } else {
            Self { value, kind: PatternKind::Exact }
        }
    }

    fn get_exact_file_match(&self, working_dir: &Path) -> Vec<PathBuf> {
        match fs::metadata(working_dir.join(self.value)) {
            Ok(meta) => match meta.is_file() {
                true => [Path::new(self.value).to_path_buf()].to_vec(),
                false => Vec::new()
            },
            Err(_) => Vec::new(),
        }
    }

    fn get_similar_file_matches(&self, working_dir: &Path) -> Vec<PathBuf> {
        let prefix = self.value;
        let full_path = working_dir.join(prefix);
        let path_parent = match full_path.is_dir() {
            true => full_path,
            false => full_path.parent().unwrap().to_owned(),
        };
        let filter_fn = |path: &Path| {
            let path_rel = path.strip_prefix(&working_dir).unwrap();
            let path_rel_str = path_rel.to_path_str();
            let is_match = path_rel_str.starts_with(prefix)
                || (path.is_dir() && prefix.starts_with(path_rel_str));
            match is_match {
                true => Some(path_rel.to_owned()),
                false => None,
            }
        };
        match self.kind {
            PatternKind::Recursive => walk_dir(&path_parent, &filter_fn),
            PatternKind::SameLevel => list_files(&path_parent, &filter_fn),
            _ => panic!("programming error. this should never be the case")
        }
    }

    /// Lists all the files in a directory matching this pattern.
    /// - If the directory does not exist, returns empty list.
    pub fn match_files(&self, working_dir: &Path) -> Vec<PathBuf> {
        if self.kind == PatternKind::Exact {
            self.get_exact_file_match(working_dir)
        } else {
            self.get_similar_file_matches(working_dir)
        }
    }

    /// Removes all the files matching the given pattern.
    /// - If the directory does not exist, returns empty list.
    pub fn remove_files(&self, working_dir: &Path) -> Vec<PathBuf> {
        let matches = self.match_files(working_dir);
        for path in matches.iter() {
            fs::remove_file(working_dir.join(path)).unwrap();
        }
        remove_empty_dirs(working_dir);
        matches
    }
}

pub trait PatternFilter {
    /// Lists all the items in the iterator matching the given pattern.
    fn filter_pattern(self, pattern: Pattern) -> Vec<String>;

    /// Explores the items in the iterator starting with the given prefix.
    /// This is ideal for directory-like exploring of flat list of strings.
    ///
    /// ## Behavior
    /// Say the iterator produces `["any", "oth", "animal/dog", "animal/cat"]`.
    /// - prefix `"an"` produces `["animal/", "any"]`, in that order.
    /// - prefix `"animal/"` produces `["cat", "dog"]`, in that order.
    fn explore_contents(self, prefix: &str) -> Vec<String>;
}

impl<I, S> PatternFilter for I
where I: Iterator<Item = S>, S: AsRef<str> {
    fn filter_pattern(self, pattern: Pattern) -> Vec<String> {
        let prefix = pattern.value;
        let pat_levels = prefix.matches('/').count();
        let iter_filtered = self.filter_map(|item| {
            let item_ref = item.as_ref();
            let is_match = match pattern.kind {
                PatternKind::Exact => item_ref == prefix,
                PatternKind::SameLevel => item_ref.starts_with(prefix)
                    && item_ref.matches('/').count() == pat_levels,
                PatternKind::Recursive => item_ref.starts_with(prefix)
            };
            match is_match {
                true => Some(item_ref.to_owned()),
                false => None
            }
        });
        iter_filtered.collect::<Vec<_>>()
    }

    fn explore_contents(self, prefix: &str) -> Vec<String> {
        let prefix_levels = prefix.matches('/').count();
        let iter_filtered = self.filter_map(|item| {
            let item_ref = item.as_ref();
            let item_levels = item_ref.matches('/').count();
            let level_match = item_levels == prefix_levels
                || item_levels == prefix_levels + 1;
            if item_ref.starts_with(prefix) && level_match {
                let mut slashes = item_ref.match_indices('/');
                let start = prefix_levels
                    .checked_sub(1)
                    .map(|val| slashes.nth(val).unwrap().0 + 1)
                    .unwrap_or(0);
                let entry = match slashes.next() {
                    Some(end) => format!("!{}", &item_ref[start..=end.0]),
                    None => item_ref[start..].to_owned(),
                };
                Some(entry)
            } else {
                None
            }
        });
        HashSet::<String>::from_iter(iter_filtered)
            .into_iter()
            .collect::<Vec<_>>()
            .into_sorted()
            .into_iter()
            .map(|val| val.trim_start_matches('!').to_owned())
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::util::ext::{PathExt, VecExt};
    use once_cell::sync::Lazy;
    use std::fs;
    use std::panic;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use super::{Pattern, PatternFilter};

    const GLOB_DIR: &'static str = "glob-test-dir";
    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        let lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        let sub_dir = root_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(root_dir.join("f1"), "content f1").unwrap();
        fs::write(root_dir.join("f2"), "content f2").unwrap();
        fs::write(sub_dir.join("f3"), "content f3").unwrap();
        let result = panic::catch_unwind(|| test());
        fs::remove_dir_all(root_dir).unwrap_or_default();
        drop(lock);
        assert!(result.is_ok())
    }

    fn map_pathbuf_to_string(original: Vec<PathBuf>) -> Vec<String> {
        original
            .into_iter()
            .map(|buf| buf.to_path_str().to_owned())
            .collect::<Vec<_>>()
    }

    #[test]
    fn should_match_files_exact() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let pattern = Pattern::from_str("f1");
            let matched = pattern.match_files(root_dir);
            assert_eq!(map_pathbuf_to_string(matched), ["f1"]);
        })
    }

    #[test]
    fn should_match_files_same_level() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let pattern = Pattern::from_str("f*");
            let matched = pattern.match_files(root_dir);
            let matched_str = map_pathbuf_to_string(matched).into_sorted();
            assert_eq!(matched_str, ["f1", "f2"]);
        })
    }

    #[test]
    fn should_get_matching_files_same_level_deep() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let pattern = Pattern::from_str("sub/*");
            let matched = pattern.match_files(root_dir);
            assert_eq!(map_pathbuf_to_string(matched), ["sub/f3"]);
        })
    }

    #[test]
    fn should_get_matching_files_recursive() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let pattern = Pattern::from_str("sub/**");
            let matched = pattern.match_files(root_dir);
            assert_eq!(map_pathbuf_to_string(matched), ["sub/f3"]);
        })
    }

    #[test]
    fn should_remove_matching_files_same_level() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let pattern = Pattern::from_str("f*");
            let matched = pattern.remove_files(root_dir);
            let matched_str = map_pathbuf_to_string(matched.into_sorted());
            assert_eq!(matched_str, ["f1", "f2"]);
            assert!(fs::read(root_dir.join("f1")).is_err());
            assert!(fs::read(root_dir.join("sub").join("f3")).is_ok());
        })
    }

    #[test]
    fn should_remove_matching_files_recursive() {
        run_test(|| {
            let glob_dir = Path::new(GLOB_DIR);
            let pattern = Pattern::from_str("**");
            let matched = pattern.remove_files(glob_dir);
            let matched_str = map_pathbuf_to_string(matched.into_sorted());
            assert_eq!(matched_str, ["f1", "f2", "sub/f3"]);
            assert!(fs::read_dir(GLOB_DIR).is_err());
        })
    }

    #[test]
    fn should_filter_matching_same_level() {
        let input = ["f1", "f2", "sub/f3"];
        let pattern = Pattern::from_str("*");
        let matches = input.into_iter().filter_pattern(pattern);
        assert_eq!(matches.into_sorted(), ["f1", "f2"]);
    }

    #[test]
    fn should_filter_matching_recursive() {
        let input = ["f1", "f2", "sub/f3"];
        let pattern = Pattern::from_str("**");
        let matches = input.into_iter().filter_pattern(pattern);
        assert_eq!(matches.into_sorted(), ["f1", "f2", "sub/f3"]);
    }

    #[test]
    fn should_explore_contents_search() {
        let input = ["any", "else", "animal/dog", "animal/cat"];
        let matches = input.into_iter().explore_contents("an");
        assert_eq!(matches, ["animal/", "any"]);
    }

    #[test]
    fn should_explore_contents_search_deep() {
        let input = ["a/b/any", "a/b/els", "a/b/animal/dog", "a/b/animal/cat"];
        let matches = input.into_iter().explore_contents("a/b/an");
        assert_eq!(matches, ["animal/", "any"]);
    }

    #[test]
    fn should_explore_contents_dir() {
        let input = ["any", "else", "animal/dog", "animal/cat"];
        let matches = input.into_iter().explore_contents("animal/");
        assert_eq!(matches, ["cat", "dog"]);
    }
}
