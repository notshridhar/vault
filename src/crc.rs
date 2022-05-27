use crate::constants::LOCK_DIR;
use crc::{Crc, CRC_32_ISCSI};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io;

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

fn compute_crc(file_path: &str) -> io::Result<u32> {
    let file_content = fs::read(file_path)?;
    Ok(CRC32.checksum(&file_content))
}

fn compute_crc_all() -> HashMap<u16, u32> {
    if let Ok(dir_entries) = fs::read_dir(LOCK_DIR) {
        dir_entries.fold(HashMap::new(), |mut accum, entry| {
            let lock_file = entry.unwrap();
            let file_name = lock_file.file_name().into_string().unwrap();
            let file_index_str = file_name.split_once('.').unwrap().0;
            if let Ok(file_index) = file_index_str.parse::<u16>() {
                let file_content = fs::read(lock_file.path()).unwrap();
                let checksum = CRC32.checksum(&file_content);
                accum.insert(file_index, checksum);
            }
            accum
        })
    } else {
        HashMap::new()
    }
}

fn read_crc_file() -> HashMap<u16, u32> {
    let crc_file = format!("{}/index.crc", LOCK_DIR);
    if let Ok(contents) = fs::read_to_string(&crc_file) {
        serde_json::from_str(&contents).unwrap()
    } else {
        HashMap::new()
    }
}

fn write_crc_file(crc_map: &HashMap<u16, u32>) -> () {
    let crc_file = format!("{}/index.crc", LOCK_DIR);
    let contents = serde_json::to_string(crc_map).unwrap();
    fs::create_dir_all(LOCK_DIR).unwrap();
    fs::write(crc_file, contents).unwrap()
}

pub fn check_crc(file_index: &u16) -> Result<u32, CrcMismatchError> {
    let file_path = format!("{}/{:0>3}.vlt", LOCK_DIR, file_index);
    let stored_crc_all = read_crc_file();
    let error = CrcMismatchError { index: file_index.to_owned() };
    if let Ok(computed_crc) = compute_crc(&file_path) {
        if let Some(stored_crc) = stored_crc_all.get(file_index) {
            if stored_crc == &computed_crc {
                Ok(computed_crc)
            } else {
                Err(error)
            }
        } else {
            Err(error)
        }
    } else {
        Err(error)
    }
}

pub fn update_crc(file_index: &u16) -> () {
    let file_path = format!("{}/{:0>3}.vlt", LOCK_DIR, file_index);
    let mut stored_crc_all = read_crc_file();
    if let Ok(computed_crc) = compute_crc(&file_path) {
        stored_crc_all.insert(file_index.to_owned(), computed_crc);
    } else {
        stored_crc_all.remove(file_index);
    }
    write_crc_file(&stored_crc_all);
}

pub fn check_crc_all() -> Result<HashMap<u16, u32>, CrcMismatchError> {
    let stored_crc = read_crc_file();
    let computed_crc = compute_crc_all();

    let added_errors = computed_crc.keys()
        .filter(|computed_key| !stored_crc.contains_key(computed_key))
        .map(|computed_key| CrcMismatchError {
            index: computed_key.to_owned(),
        });

    let removed_errors = stored_crc.keys()
        .filter(|stored_key| !computed_crc.contains_key(stored_key))
        .map(|stored_key| CrcMismatchError {
            index: stored_key.to_owned(),
        });

    let diff_errors = stored_crc.keys()
        .filter(|stored_key| {
            let stored_value = stored_crc.get(stored_key).unwrap();
            computed_crc.get(stored_key)
                .map(|computed_value| computed_value != stored_value)
                .unwrap_or(false)
        })
        .map(|stored_key| CrcMismatchError {
            index: stored_key.to_owned(),
        });

    match added_errors.chain(removed_errors).chain(diff_errors).next() {
        Some(err) => Err(err),
        None => Ok(computed_crc),
    }
}

pub fn update_crc_all() -> () {
    let computed_crc = compute_crc_all();
    write_crc_file(&computed_crc)
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CrcMismatchError {
    pub index: u16,
}
