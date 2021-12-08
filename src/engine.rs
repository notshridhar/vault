use crate::crypto;
use crate::error::VaultError;
use crate::utils;
use rpassword;
use std::fs;
use std::io::{self, Write};

type VaultResult<T> = Result<T, VaultError>;

macro_rules! get_index_path {
    ($ns:expr) => {
        format!("{}_index.vlt", $ns)
    }
}

macro_rules! get_encrypted_path {
    ($ns:expr, $num:expr) => {
        format!("{}_{:0>3}.vlt", $ns, $num)
    }
}

macro_rules! get_unlocked_path {
    ($path:expr) => {
        format!("unlock/{}", $path)
    }
}

fn prompt_secret_disappear(prompt: &str) -> io::Result<String> {
    let result = rpassword::prompt_password_stdout(prompt)?;
    print!("\x1b[1A\x1b[2K");
    io::stdout().flush()?;
    Ok(result)
}

pub fn get_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt_secret_disappear("password: ").unwrap();
    let index_path = get_index_path!(namespace);
    let index_map = crypto::decrypt_kv(&index_path, &password)?;

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        let index_val = index_map.get(&index_key).unwrap();
        let lock_file = get_encrypted_path!(namespace, index_val);
        let unlock_file = get_unlocked_path!(index_key);
        crypto::decrypt_file(&lock_file, &unlock_file, &password)?;
    }

    prompt_secret_disappear("press enter key to exit").unwrap();

    let unlock_files = get_unlocked_path!(secret_path);
    utils::remove_matching_files(&unlock_files).unwrap();

    Ok(())
}

pub fn set_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt_secret_disappear("password: ").unwrap();
    let index_path = get_index_path!(namespace);
    let mut index_map = crypto::decrypt_kv(&index_path, &password)?;

    let unlock_files = get_unlocked_path!(secret_path);
    for unlock_file in utils::get_matching_files(&unlock_files).unwrap() {
        let index_key = unlock_file.split_once('/').unwrap().1;

        let index_val = if index_map.contains_key(index_key) {
            index_map.get(index_key).unwrap().parse::<u16>().unwrap()
        } else {
            let index_val = utils::get_minimum_available_value(&index_map);
            index_map.insert(index_key.to_string(), index_val.to_string());
            index_val
        };

        let lock_file = get_encrypted_path!(namespace, index_val);
        crypto::encrypt_file(&unlock_file, &lock_file, &password)?;
    }

    crypto::encrypt_kv(&index_map, &index_path, &password).unwrap();
    
    utils::remove_matching_files(&unlock_files).unwrap();

    println!("success");
    Ok(())
}

pub fn remove_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt_secret_disappear("password: ").unwrap();
    let index_path = get_index_path!(namespace);
    let mut index_map = crypto::decrypt_kv(&index_path, &password)?;

    for index_key in utils::get_matching_keys(&index_map, secret_path) {
        let index_val = index_map.remove(&index_key).unwrap();
        
        let lock_file = get_encrypted_path!(namespace, &index_val);
        fs::remove_file(&lock_file).unwrap();
    }

    crypto::encrypt_kv(&index_map, &index_path, &password).unwrap();

    println!("success");
    Ok(())
}
