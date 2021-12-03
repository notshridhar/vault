use crate::crypto;
use crate::error::VaultError;
use crate::prompt;
use crate::utils;
use std::collections::HashMap;
use std::fs;

type VaultResult<T> = Result<T, VaultError>;

pub fn get_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt::prompt_secret_disappear("password: ").unwrap();
    let index_file_path = format!("{}_fs_index.vlt", namespace);

    let index_map = match fs::read(&index_file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        let index_val = index_map.get(&index_key).unwrap();
        let lock_file = format!("{}_fs_{:0>3}.vlt", namespace, index_val);
        let unlock_file = format!("unlock/{}", &index_key);
        crypto::decrypt_file(&lock_file, &unlock_file, &password)?;
    }

    prompt::prompt_secret_disappear("press enter key to exit").unwrap();

    let unlock_files = format!("unlock/{}", secret_path);
    utils::remove_matching_files(&unlock_files).unwrap();

    Ok(())
}

pub fn set_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt::prompt_secret_disappear("password: ").unwrap();
    let index_file_path = format!("{}_fs_index.vlt", namespace);

    let mut index_map = match fs::read(&index_file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    let unlock_files = format!("unlock/{}", secret_path);
    for unlock_file in utils::get_matching_files(&unlock_files).unwrap() {
        let index_key = unlock_file.strip_prefix("unlock/").unwrap();

        let index_val = if index_map.contains_key(index_key) {
            index_map.get(index_key).unwrap().parse::<u16>().unwrap()
        } else {
            let index_val = utils::get_minimum_available_value(&index_map);
            index_map.insert(index_key.to_string(), index_val.to_string());
            index_val
        };

        let lock_file = format!("{}_fs_{:0>3}.vlt", namespace, index_val);
        crypto::encrypt_file(&unlock_file, &lock_file, &password)?;
    }

    let index_map_enc = crypto::encrypt_kv(&index_map, &password).unwrap();
    fs::write(&index_file_path, index_map_enc).unwrap();

    utils::remove_matching_files(&unlock_files).unwrap();

    println!("success");
    Ok(())
}

pub fn rem_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt::prompt_secret_disappear("password: ").unwrap();
    let index_file_path = format!("{}_fs_index.vlt", namespace);

    let mut index_map = match fs::read(&index_file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    let index_keys = utils::get_matching_keys(&index_map, secret_path);

    if index_keys.is_empty() == false {
        let answer = prompt::prompt_input_disappear("remove? [y/N] ").unwrap();
        if answer.trim_end().to_lowercase() != "y" {
            return Ok(());
        };
    };

    for index_key in index_keys {
        let index_val = index_map.remove(&index_key).unwrap();
        let lock_file = format!("{}_fs_{:0>3}.vlt", namespace, &index_val);
        fs::remove_file(&lock_file).unwrap();
    }

    println!("success");
    Ok(())
}
