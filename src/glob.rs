use crate::util::PathExt;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

fn remove_empty_dirs<P: AsRef<Path>>(working_dir: P) -> bool {
    if let Ok(read_results) = fs::read_dir(&working_dir) {
        let is_empty = read_results.into_iter().fold(true, |accum, item| {
            let entry_path = item.unwrap().path();
            accum && if entry_path.is_dir() {
                remove_empty_dirs(entry_path)
            } else {
                let file_name = entry_path.to_filename_str();
                match [".DS_Store"].contains(&file_name) {
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
        let prefix = pattern.strip_suffix("*").unwrap();
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
            match path_rel.starts_with(prefix) {
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
