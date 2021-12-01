use itertools::Itertools;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

type Dict = HashMap<String, String>;

pub fn create_parent_dir(path: &str) -> io::Result<()> {
    if let Some(last_slash) = path.rfind('/') {
        let parent = path.split_at(last_slash).0;
        fs::create_dir_all(parent)?;
    };
    Ok(())
}

pub fn walk_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    for entry_res in fs::read_dir(path)? {
        let entry_path = entry_res?.path();
        if entry_path.is_dir() {
            let mut sub_files = walk_dir(&entry_path)?;
            all_files.append(&mut sub_files);
        } else {
            all_files.push(entry_path);
        };
    }

    Ok(all_files)
}

pub fn list_dirs(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut all_dirs = Vec::new();

    for entry_res in fs::read_dir(path)? {
        let entry_path = entry_res?.path();
        if entry_path.is_dir() {
            all_dirs.push(entry_path);
        };
    }

    Ok(all_dirs)
}

pub fn get_matching_files(path: &str) -> io::Result<Vec<String>> {
    let mut matches = Vec::new();

    let should_list = path.ends_with('*');
    let should_list_all = path.ends_with("**");
    let path = path.trim_end_matches('*');

    let last_slash = path.rfind('/').unwrap_or(0); // NOTE: edge case alert
    let path_parent = path.split_at(last_slash).0;

    let required_levels = path.matches('/').count();

    for file_path in walk_dir(Path::new(path_parent))? {
        let file_path = file_path.to_str().unwrap();
        let is_equal = file_path == path;
        let is_subpath = file_path.starts_with(path);
        let is_child = file_path.matches('/').count() == required_levels;
        if is_equal || (should_list && is_subpath && (should_list_all || is_child)) {
            matches.push(file_path.to_string());
        };
    }

    Ok(matches)
}

pub fn get_matching_dirs(path: &str) -> io::Result<Vec<String>> {
    let mut matches = Vec::new();

    let should_list = path.ends_with('*');
    let path = path.trim_end_matches('*');

    let last_slash = path.rfind('/').unwrap_or(0); // NOTE: edge case alert
    let path_parent = path.split_at(last_slash).0;

    for dir_path in list_dirs(Path::new(path_parent))? {
        let dir_path = dir_path.to_str().unwrap();
        let is_equal = dir_path == path;
        let is_subpath = dir_path.starts_with(path);
        if is_equal || (should_list && is_subpath) {
            matches.push(dir_path.to_string());
        };
    }

    Ok(matches)
}

pub fn remove_matching_files(path: &str) -> io::Result<()> {
    if path.ends_with("**") {
        for dir_path in get_matching_dirs(path).unwrap() {
            fs::remove_dir_all(&dir_path).unwrap();
        }
    };

    for file_path in get_matching_files(path).unwrap() {
        fs::remove_file(&file_path).unwrap();
    }

    Ok(())
}

pub fn get_matching_keys(map: &Dict, path: &str) -> Vec<String> {
    let mut matches = Vec::new();

    let should_list = path.ends_with('*');
    let should_list_all = path.ends_with("**");
    let path = path.trim_end_matches('*');

    let required_levels = path.matches('/').count();

    for (key, _value) in map.iter().sorted() {
        let is_equal = key == path;
        let is_subpath = key.starts_with(path);
        let is_child = key.matches('/').count() == required_levels;
        if is_equal || (should_list && is_subpath && (should_list_all || is_child)) {
            matches.push(key.to_string());
        };
    }

    matches
}

pub fn get_minimum_available_value(map: &Dict) -> u16 {
    let mut values = map
        .values()
        .map(|val| val.parse::<u16>().unwrap())
        .collect::<Vec<_>>();

    values.sort();

    let mut min_value = 1_u16;
    for value in values {
        if min_value == value {
            min_value += 1;
        } else {
            break;
        }
    }

    min_value
}
