use crate::constants::LOCK_DIR;
use crate::error::VaultError;
use crate::utils;
use crc::{Crc, CRC_32_ISCSI};
use serde_json;
use std::collections::HashMap;
use std::fs;

type CrcDict = HashMap<u16, u32>;
type VaultResult<T> = Result<T, VaultError>;

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

fn read_crc_file() -> CrcDict {
    let crc_file_path = format!("{}/index.crc", LOCK_DIR);
    match fs::read_to_string(crc_file_path) {
        Ok(contents) => serde_json::from_str::<CrcDict>(&contents).unwrap(),
        Err(_err) => HashMap::new(),
    }
}

fn write_crc_file(map: &CrcDict) -> () {
    let crc_file_path = format!("{}/index.crc", LOCK_DIR);
    let contents = serde_json::to_string(map).unwrap();
    fs::write(crc_file_path, contents).unwrap();
}

fn compute_crc(path: &str) -> CrcDict {
    let mut result = HashMap::new();

    let lock_files = format!("{}/{}", LOCK_DIR, path);
    for lock_file in utils::get_matching_files(&lock_files).unwrap() {
        let file_name = lock_file.split_once('/').unwrap().1;
        let file_index_str = file_name.split_once('.').unwrap().0;
        if let Ok(file_index) = file_index_str.parse::<u16>() {
            let file_content = fs::read(lock_file).unwrap();
            let checksum = CRC32.checksum(&file_content);
            result.insert(file_index, checksum);
        }
    }

    result
}

pub fn check_crc(path: &str) -> VaultResult<()> {
    let stored_crc = read_crc_file();
    let computed_crc = compute_crc(path);

    for stored_key in stored_crc.keys() {
        if !computed_crc.contains_key(stored_key) {
            let message = "missing file".to_string();
            return Err(VaultError::CrcMismatch(message));
        }
    }

    for computed_key in computed_crc.keys() {
        if !stored_crc.contains_key(computed_key) {
            let message = "new file".to_string();
            return Err(VaultError::CrcMismatch(message));
        }
    }

    for (stored_key, stored_value) in stored_crc {
        let computed_value = computed_crc.get(&stored_key).unwrap();
        if computed_value != &stored_value {
            let message = format!("mismatch for {}", stored_key);
            return Err(VaultError::CrcMismatch(message));
        }
    }

    Ok(())
}

pub fn update_crc() -> VaultResult<()> {
    Ok(write_crc_file(&compute_crc("**")))
}
