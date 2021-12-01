use crate::crypto;
use crate::error::VaultError;
use crate::prompt;
use crate::utils;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

type VaultResult<T> = Result<T, VaultError>;

pub fn get_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt::prompt_secret_disappear("password: ").unwrap();
    let file_path = format!("{}_kv.vlt", namespace);

    let secret_map = match fs::read(&file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    let mut lines_written = 0;
    for key in utils::get_matching_keys(&secret_map, secret_path) {
        let value = secret_map.get(&key).unwrap();
        println!("{}: {}", key, value);
        lines_written += 1;
    }

    prompt::prompt_secret_disappear("press enter key to exit").unwrap();

    // for get operation, single line is written, which can be cleared easily.
    // for list operations, the lines written can exceed the terminal height,
    // which makes it nearly impossible to clear the lines which are pushed
    // to scrollback. therefore, we attempt to clear the entire scrollback.
    match lines_written {
        0 => (),
        1 => print!("\x1b[1F\x1b[2K"),
        _ => print!("\x1b[3J\x1b[H\x1b[2J"),
    };
    io::stdout().flush().unwrap();

    Ok(())
}

pub fn set_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt::prompt_secret_disappear("password: ").unwrap();
    let file_path = format!("{}_kv.vlt", namespace);

    let mut secret_map = match fs::read(&file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    let secret = prompt::prompt_secret_disappear("secret: ").unwrap();
    let previous_secret = match secret.len() {
        0 => secret_map.remove(secret_path),
        _ => secret_map.insert(secret_path.to_string(), secret),
    };

    if previous_secret.is_some() {
        let answer = prompt::prompt_input_disappear("overwrite? [y/N] ").unwrap();
        if answer.trim_end().to_lowercase() != "y" {
            return Ok(());
        };
    };

    let map_enc = crypto::encrypt_kv(&secret_map, &password).unwrap();
    fs::write(&file_path, map_enc).unwrap();

    println!("success");
    Ok(())
}
