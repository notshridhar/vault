use crate::util::{PathExt, VecExt};
use std::fs;
use std::path::{Path, PathBuf};

/// Lists all files in the given directory.
/// - If the directory does not exist, returns empty list.
fn list_files<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(dir) {
        read_results
            .filter_map(|entry| {
                let entry_path = entry.unwrap().path();
                match entry_path.is_file() {
                    true => Some(entry_path),
                    false => None,
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Lists all files in the given directory recursively.
/// - If the directory does not exist, returns empty list.
fn walk_dir<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(dir) {
        read_results.fold(Vec::with_capacity(4), |accum, item| {
            let entry_path = item.unwrap().path();
            match entry_path.is_dir() {
                true => accum.extend_inplace(walk_dir(entry_path)),
                false => accum.push_inplace(entry_path),
            }
        })
    } else {
        Vec::new()
    }
}

/// Removes all empty directories recursively within the given directory.
/// Returns whether the given directory was completely removed.
/// Also removes some commonly found system cache files in the process.
fn remove_empty_dirs<P: AsRef<Path>>(dir: P) -> bool {
    let removables = [".DS_Store"];
    if let Ok(read_results) = fs::read_dir(&dir) {
        let is_empty = read_results.fold(true, |accum, item| {
            let entry_path = item.unwrap().path();
            let is_removed = if entry_path.is_dir() {
                remove_empty_dirs(entry_path)
            } else {
                let file_name = entry_path.to_filename_str();
                match removables.contains(&file_name) {
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

/// Lists all the files matching the given pattern in alphabetical order.
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
        let path_parent = match Path::new(prefix).parent() {
            Some(parent) => working_dir.as_ref().join(parent),
            None => working_dir.as_ref().to_path_buf(),
        };
        let file_list = match pattern.ends_with("**") {
            true => walk_dir(path_parent),
            false => list_files(path_parent),
        };
        file_list
            .into_iter()
            .filter_map(|path| {
                let path_rel = path.strip_prefix(&working_dir).unwrap();
                match path_rel.to_path_str().starts_with(prefix) {
                    true => Some(path_rel.to_owned()),
                    false => None,
                }
            })
            .collect::<Vec<_>>()
            .into_sorted()
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
/// Returns sorted list of matched files which were removed.
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
    matched_pathbufs.into_sorted()
}

/// Lists all the items in the iterator matching the given pattern
/// in alphabetical order.
///
/// ## Patterns
/// - `"pat"` matches only `"pat"`, but not `"path"`.
/// - `"pat*"` matches `"pat", "path"`, but not `"pat/some"`.
/// - `"pat**"` matches all `"pat", "path", "pat/some"`.
pub fn filter_matching<I, S>(iter: I, pattern: &str) -> Vec<String>
where I: Iterator<Item = S>, S: AsRef<str> {
    if pattern.ends_with("**") {
        let prefix = pattern.strip_suffix("**").unwrap();
        let iter_mapped = iter.filter_map(|item| {
            let item_ref = item.as_ref();
            match item_ref.starts_with(prefix) {
                true => Some(item_ref.to_owned()),
                false => None
            }
        });
        iter_mapped.collect::<Vec<_>>().into_sorted()
    } else if pattern.ends_with('*') {
        let prefix = pattern.strip_suffix('*').unwrap();
        let pat_levels = prefix.matches('/').count();
        let iter_mapped = iter.filter_map(|item| {
            let item_ref = item.as_ref();
            let item_levels = item_ref.matches('/').count();
            match item_ref.starts_with(prefix) && item_levels == pat_levels {
                true => Some(item_ref.to_owned()),
                false => None
            }
        });
        iter_mapped.collect::<Vec<_>>().into_sorted()
    } else {
        let iter_mapped = iter.filter_map(|item|
            match item.as_ref() == pattern {
                true => Some(item.as_ref().to_owned()),
                false => None
            }
        );
        iter_mapped.collect()
    }
}

#[cfg(test)]
mod test {
    use crate::util::PathExt;
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
            assert_eq!(map_pathbuf_to_string(matched), ["f1", "f2"]);
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
            assert_eq!(map_pathbuf_to_string(matched), ["f1", "f2"]);
            assert!(fs::read(root_dir.join("f1")).is_err());
            assert!(fs::read(root_dir.join("sub").join("f3")).is_ok());
        })
    }

    #[test]
    fn should_remove_matching_files_recursive() {
        run_test(|| {
            let matched = super::remove_matching_files("**", GLOB_DIR);
            assert_eq!(map_pathbuf_to_string(matched), ["f1", "f2", "sub/f3"]);
            assert!(fs::read_dir(GLOB_DIR).is_err());
        })
    }

    #[test]
    fn should_filter_matching_same_level() {
        let input_names = ["f1", "f2", "sub/f3"];
        let matching = super::filter_matching(input_names.into_iter(), "*");
        assert_eq!(matching, ["f1", "f2"]);
    }

    #[test]
    fn should_filter_matching_recursive() {
        let input_names = ["f1", "f2", "sub/f3"];
        let matching = super::filter_matching(input_names.into_iter(), "**");
        assert_eq!(matching, ["f1", "f2", "sub/f3"]);
    }
}
