use super::common::{PathExt, VecExt};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const IGNORE: &[&str] = &[".DS_Store"];

/// Lists all files in the given directory.
/// - If the directory does not exist, returns empty list.
/// - Does not match system cache files.
fn list_files<P, F>(dir: P, f: &F) -> Vec<PathBuf>
where P: AsRef<Path>, F: Fn(&Path) -> Option<PathBuf> {
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
fn walk_dir<P, F>(dir: P, f: &F) -> Vec<PathBuf>
where P: AsRef<Path>, F: Fn(&Path) -> Option<PathBuf> {
    if let Ok(read_results) = fs::read_dir(dir) {
        read_results.fold(Vec::with_capacity(4), |accum, item| {
            let entry_path = item.unwrap().path();
            let file_name = entry_path.to_filename_str();
            match f(&entry_path) {
                Some(path) => match entry_path.is_dir() {
                    true => accum.extend_inplace(walk_dir(entry_path, f)),
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
fn remove_empty_dirs<P: AsRef<Path>>(dir: P) -> bool {
    if let Ok(read_results) = fs::read_dir(&dir) {
        let is_empty = read_results.fold(true, |accum, item| {
            let entry_path = item.unwrap().path();
            let is_removed = if entry_path.is_dir() {
                remove_empty_dirs(entry_path)
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

/// Lists all the files matching the given pattern.
/// - If the directory does not exist, returns empty list.
///
/// ## Patterns
/// - `"pat"` matches only `"pat"`, but not `"path"`.
/// - `"pat*"` matches `"pat", "path"`, but not `"pat/some"`.
/// - `"pat**"` matches all `"pat", "path", "pat/some"`.
pub fn get_matching_files<P>(pattern: &str, working_dir: P) -> Vec<PathBuf>
where P: AsRef<Path> {
    if pattern.ends_with('*') {
        let prefix = pattern.trim_end_matches('*');
        let full_path = working_dir.as_ref().join(prefix);
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
        match pattern.ends_with("**") {
            true => walk_dir(path_parent, &filter_fn),
            false => list_files(path_parent, &filter_fn),
        }
    } else {
        match fs::metadata(working_dir.as_ref().join(pattern)) {
            Ok(meta) => match meta.is_file() {
                true => [Path::new(pattern).to_path_buf()].to_vec(),
                false => Vec::new()
            },
            Err(_) => Vec::new(),
        }
    }
}

/// Removes all the files matching the given pattern.
/// - If the directory does not exist, returns empty list.
///
/// ## Patterns
/// - `"pat"` matches only `"pat"`, but not `"path"`.
/// - `"pat*"` matches `"pat", "path"`, but not `"pat/some"`.
/// - `"pat**"` matches all `"pat", "path", "pat/some"`.
pub fn remove_matching_files<P>(pattern: &str, working_dir: P) -> Vec<PathBuf>
where P: AsRef<Path> {
    let matched_pathbufs = get_matching_files(pattern, &working_dir);
    let dir_ref = working_dir.as_ref();
    matched_pathbufs
        .iter()
        .for_each(|path| fs::remove_file(dir_ref.join(path)).unwrap());
    remove_empty_dirs(working_dir);
    matched_pathbufs
}

/// Lists all the items in the iterator matching the given pattern.
///
/// ## Patterns
/// - `"pat"` matches only `"pat"`, but not `"path"`.
/// - `"pat*"` matches `"pat", "path"`, but not `"pat/some"`.
/// - `"pat**"` matches all `"pat", "path", "pat/some"`.
pub fn filter_matching<I, S>(iter: I, pattern: &str) -> Vec<String>
where I: Iterator<Item = S>, S: AsRef<str> {
    let prefix = pattern.trim_end_matches('*');
    let listed = pattern.ends_with('*');
    let recursive = pattern.ends_with("**");
    let pat_levels = prefix.matches('/').count();
    let iter_filtered = iter.filter_map(|item| {
        let item_ref = item.as_ref();
        let pat_matches = match listed {
            true => item_ref.starts_with(prefix) && match recursive {
                true => true,
                false => item_ref.matches('/').count() == pat_levels
            },
            false => item_ref == prefix
        };
        match pat_matches {
            true => Some(item_ref.to_owned()),
            false => None
        }
    });
    iter_filtered.collect::<Vec<_>>()
}

/// Explores the items in the iterator starting with the given prefix.
/// This is ideal for directory-like exploring of flat list of strings.
///
/// ## Behavior
/// Say the iterator produces `["any", "else", "animal/dog", "animal/cat"]`.
/// - prefix `"an"` produces `["animal/", "any"]`, in that order.
/// - prefix `"animal/"` produces `["cat", "dog"]`, in that order.
pub fn explore_contents<I, S>(iter: I, prefix: &str) -> Vec<String>
where I: Iterator<Item = S>, S: AsRef<str> {
    // TODO: this does not belong here.
    let prefix_levels = prefix.matches('/').count();
    let iter_filtered = iter.filter_map(|item| {
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

#[cfg(test)]
mod test {
    use crate::util::common::{PathExt, VecExt};
    use once_cell::sync::Lazy;
    use std::fs;
    use std::panic;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

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
    fn should_get_matching_files_absolute() {
        run_test(|| {
            let matched = super::get_matching_files("f1", GLOB_DIR);
            assert_eq!(map_pathbuf_to_string(matched), ["f1"]);
        })
    }

    #[test]
    fn should_get_matching_files_same_level() {
        run_test(|| {
            let matched = super::get_matching_files("f*", GLOB_DIR);
            let matched_str = map_pathbuf_to_string(matched).into_sorted();
            assert_eq!(matched_str, ["f1", "f2"]);
        })
    }

    #[test]
    fn should_get_matching_files_same_level_deep() {
        run_test(|| {
            let matched = super::get_matching_files("sub/*", GLOB_DIR);
            assert_eq!(map_pathbuf_to_string(matched), ["sub/f3"]);
        })
    }

    #[test]
    fn should_get_matching_files_recursive() {
        run_test(|| {
            let matched = super::get_matching_files("sub/**", GLOB_DIR);
            assert_eq!(map_pathbuf_to_string(matched), ["sub/f3"]);
        })
    }

    #[test]
    fn should_remove_matching_files_same_level() {
        let root_dir = Path::new(GLOB_DIR);
        run_test(|| {
            let matched = super::remove_matching_files("f*", root_dir);
            let matched_str = map_pathbuf_to_string(matched.into_sorted());
            assert_eq!(matched_str, ["f1", "f2"]);
            assert!(fs::read(root_dir.join("f1")).is_err());
            assert!(fs::read(root_dir.join("sub").join("f3")).is_ok());
        })
    }

    #[test]
    fn should_remove_matching_files_recursive() {
        run_test(|| {
            let matched = super::remove_matching_files("**", GLOB_DIR);
            let matched_str = map_pathbuf_to_string(matched.into_sorted());
            assert_eq!(matched_str, ["f1", "f2", "sub/f3"]);
            assert!(fs::read_dir(GLOB_DIR).is_err());
        })
    }

    #[test]
    fn should_filter_matching_same_level() {
        let input = ["f1", "f2", "sub/f3"];
        let matches = super::filter_matching(input.into_iter(), "*");
        assert_eq!(matches.into_sorted(), ["f1", "f2"]);
    }

    #[test]
    fn should_filter_matching_recursive() {
        let input = ["f1", "f2", "sub/f3"];
        let matches = super::filter_matching(input.into_iter(), "**");
        assert_eq!(matches.into_sorted(), ["f1", "f2", "sub/f3"]);
    }

    #[test]
    fn should_explore_contents_search() {
        let input = ["any", "else", "animal/dog", "animal/cat"];
        let matches = super::explore_contents(input.into_iter(), "an");
        assert_eq!(matches, ["animal/", "any"]);
    }

    #[test]
    fn should_explore_contents_search_deep() {
        let input = ["a/b/any", "a/b/els", "a/b/animal/dog", "a/b/animal/cat"];
        let matches = super::explore_contents(input.into_iter(), "a/b/an");
        assert_eq!(matches, ["animal/", "any"]);
    }

    #[test]
    fn should_explore_contents_dir() {
        let input = ["any", "else", "animal/dog", "animal/cat"];
        let matches = super::explore_contents(input.into_iter(), "animal/");
        assert_eq!(matches, ["cat", "dog"]);
    }
}
