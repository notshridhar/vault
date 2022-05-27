mod args;
mod constants;
mod crc;
mod secret;
mod util;
mod zip;

use chrono::offset::Local;
use crate::args::{ParsedArgs, ParserError};
use crate::constants::LOCK_DIR;
use crate::crc::CrcMismatchError;
use crate::secret::SecretError;
use rpassword;
use serde_json;
use std::collections::HashMap;
use std::io::Write;

fn prompt_password() -> String {
    let pass = rpassword::prompt_password_stdout("password:").unwrap();
    print!("\x1b[1A\x1b[2K");
    std::io::stdout().flush().unwrap();
    pass
}

fn main_app() -> Result<(), VaultCliError> {
    let args_list = std::env::args().collect::<Vec<_>>();
    let args = ParsedArgs::from_args(&args_list);
    if args.get_index(1).is_none() {
        if args.get_value("version").is_some() {
            // prints version
        } else if args.get_value("help").is_some() {
            // prints help
        }
    }

    let json_result = match args.expect_index(1, "command")? {
        // "login" => { /* login */ }
        // "get-file" => { /* write to unencrypted file */ },
        // "set-file" => { /* set from unencrypted file */ },
        "get" => {
            let path = args.expect_index(2, "secret_path")?;
            let password = args.get_value("password").map_or_else(
                prompt_password, |pass| pass.to_owned());
            let info = secret::get_secret(path, &password)?;
            serde_json::to_string(&info).unwrap()
        }
        "set" => {
            let path = args.expect_index(2, "secret_path")?;
            let contents_raw = args.expect_index(3, "contents")?;
            let contents = &contents_raw.replace("\\n", "\n");
            let password = args.get_value("password").map_or_else(
                prompt_password, |pass| pass.to_owned());
            let info = secret::set_secret(path, contents, &password)?;
            serde_json::to_string(&info).unwrap()
        }
        "rm" => {
            let path = args.expect_index(2, "secret_path")?;
            let password = args.get_value("password").map_or_else(
                prompt_password, |pass| pass.to_owned());
            let info = secret::remove_secret(path, &password)?;
            serde_json::to_string(&info).unwrap()
        }
        "ls" => {
            let pattern = args.get_index(2).unwrap_or("");
            let password = args.get_value("password").map_or_else(
                prompt_password, |pass| pass.to_owned());
            let info = if args.get_value("recursive").is_some() {
                secret::list_secret_paths_recursive(pattern, &password)
            } else {
                secret::list_secret_paths(pattern, &password)                
            }?;
            serde_json::to_string(&info).unwrap()
        }
        "crc" => {
            if args.get_value("force-update").is_some() {
                crc::update_crc_all()
            } else {
                crc::check_crc_all()?;
            }
            "{}".to_owned()
        }
        "zip" => {
            let date_stamp = Local::now().format("%Y%m%d");
            let archive_name = format!("vault-{}.zip", date_stamp);
            zip::zip_dirs(&archive_name, &[LOCK_DIR, "vault"]).unwrap();
            "{}".to_owned()
        }
        _ => Err(ParserError::Invalid { key: "command".to_owned() })?
    };

    Ok(println!("{}", json_result))
}

fn main() -> () {
    if let Err(err) = main_app() {
        eprintln!("{}", serde_json::to_string(&err).unwrap());
    }
}

pub type VaultCliError = HashMap<String, String>;

impl From<ParserError> for VaultCliError {
    fn from(error: ParserError) -> Self {
        match error {
            ParserError::Missing { key } => Self::from([
                ("error".to_owned(), "arg_missing".to_owned()),
                ("key".to_owned(), key),
            ]),
            ParserError::Invalid { key } => Self::from([
                ("error".to_owned(), "arg_invalid".to_owned()),
                ("key".to_owned(), key),
            ]),
        }
    }
}

impl From<CrcMismatchError> for VaultCliError {
    fn from(error: CrcMismatchError) -> Self {
        Self::from([
            ("error".to_owned(), "crc_mismatch".to_owned()),
            ("index".to_owned(), error.index.to_string()),
        ])
    }
}

impl From<SecretError> for VaultCliError {
    fn from(error: SecretError) -> Self {
        match error {
            SecretError::CrcMismatch { index } => Self::from([
                ("error".to_owned(), "crc_mismatch".to_owned()),
                ("index".to_owned(), index.to_string()),
            ]),
            SecretError::IncorrectPassword => Self::from([
                ("error".to_owned(), "incorrect_password".to_owned()),
            ]),
            SecretError::NonExistentPath => Self::from([
                ("error".to_owned(), "non_existent_path".to_owned()),
            ]),
        }
    }
}
