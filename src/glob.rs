use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn walk_dir(path: &str) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(path) {
        let mut found_files = Vec::with_capacity(4);
        for entry_res in read_results {
            let entry_path = entry_res.unwrap().path();
            if entry_path.is_dir() {
                let entry_path_str = entry_path.to_str().unwrap();
                found_files.extend(walk_dir(entry_path_str))
            } else {
                found_files.push(entry_path);
            }
        }
        found_files
    } else {
        Vec::new()
    }
}

fn list_files(path: &str) -> Vec<PathBuf> {
    if let Ok(read_results) = fs::read_dir(path) {
        read_results.filter_map(|path| {
            let entry_path = path.unwrap().path();
            match entry_path.is_file() {
                true => Some(entry_path),
                false => None,
            }
        }).collect()
    } else {
        Vec::new()
    }
}

pub fn get_matching_files(pattern: &str, working_dir: &str) -> Vec<String> {
    if pattern.ends_with("*") {
        let prefix = pattern.strip_suffix("*").unwrap();
        let path_parent = match Path::new(prefix).parent() {
            Some(parent) => Path::new(working_dir).join(parent),
            None => Path::new(working_dir).to_path_buf()
        };
        let file_list = match pattern.ends_with("**") {
            true => walk_dir(path_parent.to_str().unwrap()),
            false => list_files(path_parent.to_str().unwrap()),
        };
        file_list.into_iter().filter_map(|path| {
            let path_str = path
                .strip_prefix(working_dir).unwrap()
                .to_str().unwrap();
            match path_str.starts_with(prefix) {
                true => Some(path_str.to_owned()),                
                false => None,
            }
        }).collect()
    } else {
        match fs::metadata(Path::new(working_dir).join(pattern)) {
            Ok(meta) => match meta.is_file() {
                true => [pattern.to_owned()].to_vec(),
                false => Vec::new()
            },
            Err(_) => Vec::new()
        }
    }
}

pub fn filter_matching<'a, I, S>(iter: I, pattern: &str) -> Vec<String>
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
                Some(item_ref
                    .split('/')
                    .take(pat_levels + 1)
                    .collect::<Vec<_>>()
                    .join("/") + dir_suffix)
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
