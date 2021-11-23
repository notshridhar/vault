mod crypto;

use rpassword::prompt_password_stdout;
use std::collections::HashMap;
use std::env::args;
use std::fs;
use std::io::{self, Write};
use std::process;

type OpResult = Result<String, String>;

fn prompt_input_stdout(prompt: &str) -> io::Result<String> {
    let mut answer = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut answer)?;
    Ok(answer)
}

fn get_vault_secret() -> OpResult {
    let full_path = args().nth(2).expect("missing secret path");
    let (namespace, path) = full_path.split_once("://").unwrap();

    let password = prompt_password_stdout("password: ").unwrap();
    let file_path = format!("{}_kv.raw", namespace);

    let secret_map = match fs::read(&file_path) {
        Ok(raw) => match crypto::decrypt_kv(&raw, &password) {
            Ok(map) => map,
            Err(_err) => return Err("incorrect password".to_string()),
        },
        Err(_err) => HashMap::new(),
    };

    let secret = match secret_map.get(path) {
        Some(secret) => secret,
        None => return Err("path does not exist".to_string()),
    };

    Ok(format!("secret={}", secret))
}

fn set_vault_secret() -> OpResult {
    let full_path = args().nth(2).expect("missing secret path");
    let (namespace, path) = full_path.split_once("://").unwrap();

    let password = prompt_password_stdout("password: ").unwrap();
    let file_path = format!("{}_kv.raw", namespace);

    let mut secret_map = match fs::read(&file_path) {
        Ok(raw) => match crypto::decrypt_kv(&raw, &password) {
            Ok(map) => map,
            Err(_err) => return Err("incorrect password".to_string()),
        },
        Err(_err) => HashMap::new(),
    };

    let secret = prompt_password_stdout("secret: ").unwrap();
    let previous_secret = match secret.len() {
        0 => secret_map.remove(path),
        _ => secret_map.insert(path.to_string(), secret),
    };

    if previous_secret.is_some() {
        let answer = prompt_input_stdout("overwrite? [y/N] ").unwrap();
        if answer.trim_end().to_lowercase() != "y" {
            return Ok("no changes".to_string());
        };
    };

    match crypto::encrypt_kv(&secret_map, &password) {
        Ok(raw) => fs::write(&file_path, raw).unwrap(),
        Err(_err) => return Err("encryption error".to_string()),
    };

    Ok("set secret".to_string())
}

fn main() -> () {
    let subcommand = args().nth(1).expect("missing subcommand");

    let result = match subcommand.as_str() {
        "get" => get_vault_secret(),
        "set" => set_vault_secret(),
        _ => Err("unknown command".to_string()),
    };

    match result {
        Ok(message) => {
            println!("success: {}", message);
            process::exit(0);
        }
        Err(message) => {
            println!("failure: {}", message);
            process::exit(1);
        }
    };
}
