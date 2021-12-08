mod crypto;
mod engine;
mod error;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::VaultError;
use std::env;

type VaultResult<T> = Result<T, VaultError>;

fn main_app() -> VaultResult<()> {
    let mut args = env::args();
    let _app_path = args.next().ok_or(VaultError::InvalidPath)?;
    let command = args.next().ok_or(VaultError::InvalidPath)?;
    let full_path = args.next().ok_or(VaultError::InvalidPath)?;

    let mut split_path = full_path.split("::");
    let namespace = split_path.next().ok_or(VaultError::InvalidPath)?;
    let secret_path = split_path.next().ok_or(VaultError::InvalidPath)?;

    match command.as_str() {
        "get" => engine::get_secret(namespace, secret_path),
        "set" => engine::set_secret(namespace, secret_path),
        "rem" => engine::remove_secret(namespace, secret_path),
        _ => Err(VaultError::InvalidCommand),
    }
}

fn main() -> () {
    main_app().unwrap_or_else(|err| eprintln!("failure: {}", err));
}
