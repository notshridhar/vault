use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

type Dict = HashMap<String, String>;

pub fn walk_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    let read_results = match fs::read_dir(path) {
        Ok(res) => res,
        Err(_err) => return Ok(Vec::new()),
    };

    for entry_res in read_results {
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

    let read_results = match fs::read_dir(path) {
        Ok(res) => res,
        Err(_err) => return Ok(Vec::new()),
    };

    for entry_res in read_results {
        let entry_path = entry_res?.path();
        if entry_path.is_dir() {
            all_dirs.push(entry_path);
        };
    }

    Ok(all_dirs)
}

pub fn get_parent_dir(path: &str) -> &str {
    let last_slash = path.rfind('/').unwrap_or(0);
    path.split_at(last_slash).0
}

pub fn get_matching_files(path: &str) -> io::Result<Vec<String>> {
    let mut matches = Vec::new();

    let should_list = path.ends_with('*');
    let should_list_all = path.ends_with("**");
    let path = path.trim_end_matches('*');

    let required_levels = path.matches('/').count();

    for file_path in walk_dir(Path::new(get_parent_dir(path)))? {
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

    for dir_path in list_dirs(Path::new(get_parent_dir(path)))? {
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
    if path.ends_with("/**") {
        fs::remove_dir_all(get_parent_dir(path))?;
    } else if path.ends_with("**") {
        for dir_path in get_matching_dirs(path)? {
            fs::remove_dir_all(&dir_path)?;
        }
    };

    for file_path in get_matching_files(path)? {
        fs::remove_file(&file_path)?;
    }

    Ok(())
}

pub fn get_matching_keys(map: &Dict, pat: &str) -> Vec<String> {
    let mut matches = Vec::new();

    let should_list = pat.ends_with('*');
    let should_list_all = pat.ends_with("**");
    let pat = pat.trim_end_matches('*');

    let required_levels = pat.matches('/').count();

    for (key, _value) in map {
        let is_equal = key == pat;
        let is_subpath = key.starts_with(pat);
        let is_child = key.matches('/').count() == required_levels;
        if is_equal || (should_list && is_subpath && (should_list_all || is_child)) {
            matches.push(key.to_string());
        };
    }

    matches.sort();

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
