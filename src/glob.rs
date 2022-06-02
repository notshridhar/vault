use crate::util::PathExt;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn list_files<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(path) {
        read_results.filter_map(|entry| {
            let entry_path = entry.unwrap().path();
            match entry_path.is_file() {
                true => Some(entry_path),
                false => None,
            }
        }).collect()
    } else {
        Vec::new()
    }
}

fn walk_dir<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(path) {
        let mut found_files = Vec::with_capacity(4);
        for entry_res in read_results {
            let entry_path = entry_res.unwrap().path();
            match entry_path.is_dir() {
                true => found_files.extend(walk_dir(entry_path)),
                false => found_files.push(entry_path),
            }
        }
        found_files
    } else {
        Vec::new()
    }
}

fn remove_empty_dirs<P: AsRef<Path>>(working_dir: P) -> bool {
    let removables = [".DS_Store"];
    if let Ok(read_results) = fs::read_dir(&working_dir) {
        let is_empty = read_results.into_iter().fold(true, |accum, item| {
            let entry_path = item.unwrap().path();
            accum && if entry_path.is_dir() {
                remove_empty_dirs(entry_path)
            } else {
                let file_name = entry_path.to_filename_str();
                match removables.contains(&file_name) {
                    true => fs::remove_file(entry_path).is_ok(),
                    false => false,
                }
            }
        });
        fs::remove_dir(working_dir).unwrap_or_default();
        is_empty
    } else {
        true
    }
}

pub fn get_matching_files<P>(pattern: &str, working_dir: P) -> Vec<PathBuf>
where P: AsRef<Path> {
    if pattern.ends_with("*") {
        let prefix = pattern.trim_end_matches("*");
        let path_parent = match Path::new(prefix).parent() {
            Some(parent) => working_dir.as_ref().join(parent),
            None => working_dir.as_ref().to_path_buf(),
        };
        let file_list = match pattern.ends_with("**") {
            true => walk_dir(path_parent),
            false => list_files(path_parent),
        };
        file_list.into_iter().filter_map(|path| {
            let path_rel = path.strip_prefix(&working_dir).unwrap();
            match path_rel.to_path_str().starts_with(prefix) {
                true => Some(path_rel.to_owned()),
                false => None,
            }
        }).collect()
    } else {
        match fs::metadata(working_dir.as_ref().join(pattern)) {
            Ok(meta) => match meta.is_file() {
                true => [Path::new(pattern).to_path_buf()].to_vec(),
                false => Vec::new()
            },
            Err(_) => Vec::new()
        }
    }
}

pub fn remove_matching_files<P>(pattern: &str, working_dir: P) -> Vec<PathBuf>
where P: AsRef<Path> {
    let matched_pathbufs = get_matching_files(pattern, &working_dir);
    matched_pathbufs.iter().for_each(|path| {
        fs::remove_file(working_dir.as_ref().join(path)).unwrap()
    });
    remove_empty_dirs(working_dir);
    matched_pathbufs
}

pub fn filter_matching<I, S>(iter: I, pattern: &str) -> Vec<String>
where I: Iterator<Item = S>, S: AsRef<str> {
    if pattern.ends_with("**") {
        let prefix = pattern.strip_suffix("**").unwrap();
        iter.filter_map(|item| {
            let item_ref = item.as_ref();
            match item_ref.starts_with(prefix) {
                true => Some(item_ref.to_owned()),
                false => None
            }
        }).collect()
    } else if pattern.ends_with("*") {
        let prefix = pattern.strip_suffix("*").unwrap();
        let pat_levels = prefix.matches('/').count();
        let set_iter = iter.filter_map(|item| {
            let item_ref = item.as_ref();
            if item_ref.starts_with(prefix) {
                let item_levels = item_ref.matches('/').count();
                let dir_suffix = match item_levels == pat_levels {
                    true => "",
                    false => "/",
                };
                let item_display = item_ref
                    .split('/')
                    .take(pat_levels + 1)
                    .collect::<Vec<_>>()
                    .join("/") + dir_suffix;
                Some(item_display)
            } else {
                None
            }
        });
        HashSet::<String>::from_iter(set_iter).into_iter().collect()
    } else {
        iter.filter_map(|item| {
            let item_ref = item.as_ref();
            match item_ref == pattern {
                true => Some(item_ref.to_owned()),
                false => None
            }
        }).collect()
    }
}

#[cfg(test)]
mod test {
    use crate::util::{VecExt, PathExt};
    use once_cell::sync::Lazy;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    const GLOB_DIR: &'static str = "glob-test-dir";
    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn create_test_files(root_dir: &Path) -> () {
        let sub_dir = root_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(root_dir.join("f1"), "content f1").unwrap();
        fs::write(root_dir.join("f2"), "content f2").unwrap();
        fs::write(sub_dir.join("f3"), "content f3").unwrap();
    }

    fn map_pathbuf_to_string(original: Vec<PathBuf>) -> Vec<String> {
        original
            .into_iter()
            .map(|buf| buf.to_path_str().to_owned())
            .collect::<Vec<_>>()
            .into_sorted()
    }

    #[test]
    fn should_get_matching_files_absolute() {
        let _lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        create_test_files(root_dir);
        let matched = super::get_matching_files("f1", root_dir);
        let matched_str = map_pathbuf_to_string(matched);
        assert_eq!(matched_str, ["f1"]);
        fs::remove_dir_all(root_dir).unwrap();
    }

    #[test]
    fn should_get_matching_files_same_level() {
        let _lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        create_test_files(root_dir);
        let matched = super::get_matching_files("f*", root_dir);
        let matched_str = map_pathbuf_to_string(matched);
        assert_eq!(matched_str, ["f1", "f2"]);
        fs::remove_dir_all(root_dir).unwrap();
    }

    #[test]
    fn should_get_matching_files_recursive() {
        let _lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        create_test_files(root_dir);
        let matched = super::get_matching_files("**", root_dir);
        let matched_str = map_pathbuf_to_string(matched);
        assert_eq!(matched_str, ["f1", "f2", "sub/f3"]);
        fs::remove_dir_all(root_dir).unwrap();
    }

    #[test]
    fn should_remove_matching_files_same_level() {
        let _lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        create_test_files(root_dir);
        let matched = super::remove_matching_files("f*", root_dir);
        let matched_str = map_pathbuf_to_string(matched);
        assert_eq!(matched_str, ["f1", "f2"]);
        assert!(fs::read(root_dir.join("f1")).is_err());
        assert!(fs::read(root_dir.join("sub").join("f3")).is_ok());
        fs::remove_dir_all(root_dir).unwrap();
    }

    #[test]
    fn should_remove_matching_files_recursive() {
        let _lock = DIR_LOCK.lock().unwrap();
        let root_dir = Path::new(GLOB_DIR);
        create_test_files(root_dir);
        let matched = super::remove_matching_files("**", root_dir);
        let matched_str = map_pathbuf_to_string(matched);
        assert_eq!(matched_str, ["f1", "f2", "sub/f3"]);
        assert!(fs::read_dir(root_dir).is_err());
    }

    #[test]
    fn should_filter_matching_same_level() {
        let input_names = ["f1", "f2", "sub/f3"];
        let matching = super::filter_matching(input_names.into_iter(), "*");
        assert_eq!(matching.into_sorted(), ["f1", "f2", "sub/"]);
    }

    #[test]
    fn should_filter_matching_recursive() {
        let input_names = ["f1", "f2", "sub/f3"];
        let matching = super::filter_matching(input_names.iter(), "**");
        assert_eq!(matching.into_sorted(), input_names);
    }
}
