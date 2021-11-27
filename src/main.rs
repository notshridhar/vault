mod crypto;
mod error;

use error::VaultError;
use itertools::Itertools;
use rpassword::prompt_password_stdout;
use std::collections::HashMap;
use std::env::args;
use std::fs;
use std::io::{self, Write};

type VaultResult<T> = Result<T, VaultError>;

fn prompt_input_stdout(prompt: &str) -> io::Result<String> {
    let mut answer = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut answer)?;
    Ok(answer)
}

fn get_vault_kv_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt_password_stdout("password: ").unwrap();
    let file_path = format!("{}_kv.raw", namespace);

    let secret_map = match fs::read(&file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    if secret_path.ends_with('*') {
        let should_list_all = secret_path.ends_with("**");
        let secret_path = secret_path.trim_end_matches('*');
        let required_levels = secret_path.matches('/').count();
        for (key, value) in secret_map.iter().sorted() {
            let is_equal = key == secret_path;
            let is_subpath = key.starts_with(secret_path);
            let is_child = key.matches('/').count() == required_levels;
            if is_equal || (is_subpath && (should_list_all || is_child)) {
                println!("{}: {}", key, value);
            };
        }
    } else {
        if let Some(value) = secret_map.get(secret_path) {
            println!("{}: {}", secret_path, value);
        };
    };

    Ok(())
}

fn set_vault_kv_secret(namespace: &str, secret_path: &str) -> VaultResult<()> {
    let password = prompt_password_stdout("password: ").unwrap();
    let file_path = format!("{}_kv.raw", namespace);

    let mut secret_map = match fs::read(&file_path) {
        Ok(raw) => crypto::decrypt_kv(&raw, &password)?,
        Err(_err) => HashMap::new(),
    };

    let secret = prompt_password_stdout("secret: ").unwrap();
    let previous_secret = match secret.len() {
        0 => secret_map.remove(secret_path),
        _ => secret_map.insert(secret_path.to_string(), secret),
    };

    if previous_secret.is_some() {
        let answer = prompt_input_stdout("overwrite? [y/N] ").unwrap();
        if answer.trim_end().to_lowercase() != "y" {
            println!("no changes");
            return Ok(());
        };
    };

    let map_enc = crypto::encrypt_kv(&secret_map, &password).unwrap();
    fs::write(&file_path, map_enc).unwrap();

    println!("set secret");
    Ok(())
}

fn get_vault_fs_secret(_namespace: &str, _secret_path: &str) -> VaultResult<()> {
    Ok(())
}

fn set_vault_fs_secret(_namespace: &str, _secret_path: &str) -> VaultResult<()> {
    Ok(())
}

fn main_app() -> VaultResult<()> {
    let command = args().nth(1).ok_or(VaultError::InvalidCommand)?;
    let full_path = args().nth(2).ok_or(VaultError::InvalidCommand)?;

    let split_path = full_path.split("::").collect::<Vec<_>>();
    let namespace = split_path.get(0).ok_or(VaultError::InvalidPath)?;
    let secret_type = split_path.get(1).ok_or(VaultError::InvalidPath)?;
    let secret_path = split_path.get(2).ok_or(VaultError::InvalidPath)?;

    match command.as_str() {
        "get" => match secret_type.to_owned() {
            "kv" => get_vault_kv_secret(namespace, secret_path),
            "fs" => get_vault_fs_secret(namespace, secret_path),
            _ => Err(VaultError::InvalidCommand),
        },
        "set" => match secret_type.to_owned() {
            "kv" => set_vault_kv_secret(namespace, secret_path),
            "fs" => set_vault_fs_secret(namespace, secret_path),
            _ => Err(VaultError::InvalidCommand),
        },
        _ => Err(VaultError::InvalidCommand),
    }
}

fn main() -> () {
    match main_app() {
        Ok(_) => (),
        Err(err) => eprintln!("failure: {}", err),
    }
}
