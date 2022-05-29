use crate::constants::LOCK_DIR;
use crc::{Crc, CRC_32_ISCSI};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

fn compute_crc<P: AsRef<Path>>(path: P) -> io::Result<u32> {
    let file_content = fs::read(path)?;
    Ok(CRC32.checksum(&file_content))
}

fn compute_crc_all() -> HashMap<String, u32> {
    if let Ok(dir_entries) = fs::read_dir(LOCK_DIR) {
        dir_entries.fold(HashMap::new(), |mut accum, entry| {
            let file_entry = entry.unwrap();
            let file_name = file_entry.file_name();
            if file_name != "index.crc" {
                let file_content = fs::read(&file_name).unwrap();
                let checksum = CRC32.checksum(&file_content);
                accum.insert(file_name.into_string().unwrap(), checksum);
            }
            accum
        })
    } else {
        HashMap::new()
    }
}

fn read_crc_file() -> HashMap<String, u32> {
    let crc_file_path = Path::new(LOCK_DIR).join("index.crc");
    match fs::read_to_string(crc_file_path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap(),
        Err(_) => HashMap::new(),
    }
}

fn write_crc_file(crc_map: &HashMap<String, u32>) -> () {
    let crc_file = Path::new(LOCK_DIR).join("index.crc");
    let contents = serde_json::to_string(crc_map).unwrap();
    fs::create_dir_all(LOCK_DIR).unwrap();
    fs::write(crc_file, contents).unwrap()
}

pub fn check_crc<P: AsRef<Path>>(path: P) -> Result<u32, CrcMismatchError> {
    let stored_crc_all = read_crc_file();
    let path_str = path.as_ref().to_str().unwrap();
    match compute_crc(&path) {
        Ok(computed_crc) => match stored_crc_all.get(path_str) {
            Some(stored_crc) => match stored_crc == &computed_crc {
                true => Ok(computed_crc),
                false => Err(CrcMismatchError::new(path_str)),
            }
            None => Err(CrcMismatchError::new(path_str)),
        }
        Err(_) => Err(CrcMismatchError::new(path_str)),
    }
}

pub fn update_crc<P: AsRef<Path>>(path: P) -> () {
    let mut stored_crc = read_crc_file();
    let path_str = path.as_ref().to_str().unwrap();
    match compute_crc(&path) {
        Ok(crc) => stored_crc.insert(path_str.to_owned(), crc),
        Err(_) => stored_crc.remove(path_str),
    };
    write_crc_file(&stored_crc);
}

pub fn check_crc_all() -> Result<HashMap<String, u32>, CrcMismatchError> {
    let stored_crc = read_crc_file();
    let computed_crc = compute_crc_all();

    let added_errors = computed_crc.keys().filter_map(|computed_key| {
        match stored_crc.contains_key(computed_key) {
            true => None,
            false => Some(CrcMismatchError::new(computed_key)),
        }
    });

    let diff_errors = stored_crc.keys().filter_map(|stored_key| {
        let stored_value = stored_crc.get(stored_key).unwrap();
        match computed_crc.get(stored_key) {
            Some(computed_value) if computed_value == stored_value => None,
            _ => Some(CrcMismatchError::new(stored_key)),
        }
    });

    match added_errors.chain(diff_errors).next() {
        Some(err) => Err(err),
        None => Ok(computed_crc),
    }
}

pub fn update_crc_all() -> () {
    let computed_crc = compute_crc_all();
    write_crc_file(&computed_crc)
}

#[derive(Debug, PartialEq)]
pub struct CrcMismatchError {
    pub file_path: String,
}

impl CrcMismatchError {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { file_path: path.into() }
    }
}
