use crate::constants::{LOCK_DIR, UNLOCK_DIR};
use crate::crc;
use crate::crypto;
use crate::error::VaultError;
use crate::utils;
use std::fs;

type VaultResult<T> = Result<T, VaultError>;

pub fn get_secret(secret_path: &str, password: &str) -> VaultResult<()> {
    crc::check_crc()?;
    
    let index_file_path = format!("{}/index.vlt", LOCK_DIR);
    let index_map = crypto::decrypt_kv(&index_file_path, password)?;

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        let index_val = index_map.get(&index_key).unwrap();
        let lock_file = format!("{}/{:0>3}.vlt", LOCK_DIR, index_val);
        let unlock_file = format!("{}/{}", UNLOCK_DIR, index_key);
        crypto::decrypt_file(&lock_file, &unlock_file, password)?;
        println!("- {}", index_key);
    }

    Ok(())
}

pub fn set_secret(secret_path: &str, password: &str) -> VaultResult<()> {
    crc::check_crc()?;

    let index_file_path = format!("{}/index.vlt", LOCK_DIR);
    let mut index_map = crypto::decrypt_kv(&index_file_path, password)?;

    let unlock_files = format!("{}/{}", UNLOCK_DIR, secret_path);
    for unlock_file in utils::get_matching_files(&unlock_files).unwrap() {
        let index_key = unlock_file.split_once('/').unwrap().1;

        let index_val = if index_map.contains_key(index_key) {
            index_map.get(index_key).unwrap().parse::<u16>().unwrap()
        } else {
            let index_val = utils::get_minimum_available_value(&index_map);
            index_map.insert(index_key.to_string(), index_val.to_string());
            index_val
        };

        let lock_file = format!("{}/{:0>3}.vlt", LOCK_DIR, index_val);
        crypto::encrypt_file(&unlock_file, &lock_file, password)?;
        println!("- {}", index_key);
    }

    crypto::encrypt_kv(&index_map, &index_file_path, password).unwrap();

    utils::remove_matching_files(&unlock_files).unwrap();

    crc::update_crc()
}

pub fn remove_secret(secret_path: &str, password: &str) -> VaultResult<()> {
    crc::check_crc()?;

    let index_file_path = format!("{}/index.vlt", LOCK_DIR);
    let mut index_map = crypto::decrypt_kv(&index_file_path, password)?;

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        let index_val = index_map.remove(&index_key).unwrap();

        let lock_file = format!("{}/{:0>3}.vlt", LOCK_DIR, &index_val);
        fs::remove_file(&lock_file).unwrap();
        println!("- {}", index_key);
    }

    crypto::encrypt_kv(&index_map, &index_file_path, password).unwrap();

    crc::update_crc()
}

pub fn list_secrets(secret_path: &str, password: &str) -> VaultResult<()> {
    let index_file_path = format!("{}/index.vlt", LOCK_DIR);
    let index_map = crypto::decrypt_kv(&index_file_path, password)?;

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        println!("- {}", index_key);
    }

    Ok(())
}

pub fn show_secret(secret_path: &str, password: &str) -> VaultResult<()> {
    let index_file_path = format!("{}/index.vlt", LOCK_DIR);
    let index_map = crypto::decrypt_kv(&index_file_path, password)?;

    if let Some(index_val) = index_map.get(secret_path) {
        let lock_file = format!("{}/{:0>3}.vlt", LOCK_DIR, &index_val);
        let file_content = crypto::decrypt_file_content(&lock_file, password)?;
        println!("{}", file_content);
    }

    Ok(())
}

pub fn check_crc() -> VaultResult<()> {
    crc::check_crc()?;
    println!("success");
    Ok(())
}

pub fn update_crc() -> VaultResult<()> {
    crc::update_crc()?;
    println!("success");
    Ok(())
}
