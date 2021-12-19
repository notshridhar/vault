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

type VaultResult<T> = Result<T, VaultError>;

fn main_app() -> VaultResult<()> {
    let mut args = env::args();
    let _app_path = args.next().ok_or(VaultError::InvalidPath)?;
    let command = args.next().ok_or(VaultError::InvalidPath)?;
    let secret_path = args.next().ok_or(VaultError::InvalidPath)?;

    let password = rpassword::prompt_password_stdout("password: ").unwrap();
    print!("\x1b[1A\x1b[2K");
    io::stdout().flush().unwrap();

    match command.as_str() {
        "get" => engine::get_secret(&secret_path, &password),
        "set" => engine::set_secret(&secret_path, &password),
        "rem" => engine::remove_secret(&secret_path, &password),
        _ => Err(VaultError::InvalidCommand),
    }
}

fn main() -> () {
    match main_app() {
        Ok(_res) => println!("success"),
        Err(err) => eprintln!("failure: {}", err)
    }
}
