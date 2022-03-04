mod constants;
mod crc;
mod crypto;
mod engine;
mod error;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::VaultError;
use rpassword;
use std::env;
use std::io::{self, Write};
use std::path::Path;

type VaultResult<T> = Result<T, VaultError>;

fn main_app() -> VaultResult<()> {
    let mut args = env::args();
    let _app_path = args.next().ok_or(VaultError::InvalidCommand)?;
    let command = args.next().ok_or(VaultError::InvalidCommand)?;
    let secret_path = args.next().ok_or(VaultError::InvalidCommand)?;
    let mut password = String::new();
    let mut current_dir = String::new();

    let mut capture_password = false;
    let mut capture_current_dir = false;
    for current_arg in args {
        if capture_password {
            password = current_arg;
            capture_password = false;
        } else if capture_current_dir {
            current_dir = current_arg;
            capture_current_dir = false;
        } else if current_arg == "--password" {
            capture_password = true;
        } else if current_arg == "--current-dir" {
            capture_current_dir = true;
        } else if current_arg.starts_with("--password=") {
            password = current_arg.split_once('=').unwrap().1.to_string();
        } else if current_arg.starts_with("--current-dir=") {
            current_dir = current_arg.split_once('=').unwrap().1.to_string();
        } else {
            Err(VaultError::InvalidCommand)?;
        }
    }

    if capture_password || capture_current_dir {
        Err(VaultError::InvalidCommand)?;
    }

    if !command.starts_with("crc-") && password.is_empty() {
        password = rpassword::prompt_password_stdout("password: ").unwrap();
        print!("\x1b[1A\x1b[2K");
        io::stdout().flush().unwrap();
    }

    if !current_dir.is_empty() {
        let current_dir_path = Path::new(&current_dir);
        env::set_current_dir(current_dir_path).unwrap();
    };

    match command.as_str() {
        "get" => engine::get_secret(&secret_path, &password),
        "set" => engine::set_secret(&secret_path, &password),
        "rem" => engine::remove_secret(&secret_path, &password),
        "list" => engine::list_secrets(&secret_path, &password),
        "show" => engine::show_secret(&secret_path, &password),
        "crc-check" => engine::check_crc(&secret_path),
        "crc-update" => engine::update_crc(),
        _ => Err(VaultError::InvalidCommand),
    }?;

    Ok(())
}

fn main() -> () {
    if let Err(err) = main_app() {
        eprintln!("failure: {}", err);
    }
}
