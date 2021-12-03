mod crypto;
mod engine_fs;
mod engine_kv;
mod error;
mod prompt;
mod utils;

#[cfg(test)]
mod tests;

use crate::error::VaultError;
use std::env;

type VaultResult<T> = Result<T, VaultError>;

fn main_app() -> VaultResult<()> {
    let mut args = env::args();
    let _app_path = args.next().ok_or(VaultError::InvalidPath)?;
    let engine = args.next().ok_or(VaultError::InvalidPath)?;
    let command = args.next().ok_or(VaultError::InvalidPath)?;
    let full_path = args.next().ok_or(VaultError::InvalidPath)?;

    let mut split_path = full_path.split("::");
    let namespace = split_path.next().ok_or(VaultError::InvalidPath)?;
    let secret_path = split_path.next().ok_or(VaultError::InvalidPath)?;

    match engine.as_str() {
        "kv" => match command.as_str() {
            "get" => engine_kv::get_secret(namespace, secret_path),
            "set" => engine_kv::set_secret(namespace, secret_path),
            _ => Err(VaultError::InvalidCommand),
        },
        "fs" => match command.as_str() {
            "get" => engine_fs::get_secret(namespace, secret_path),
            "set" => engine_fs::set_secret(namespace, secret_path),
            "rem" => engine_fs::rem_secret(namespace, secret_path),
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
