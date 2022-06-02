mod args;
mod constants;
mod crc;
mod crypto;
mod glob;
mod secret;
mod util;
mod zip;

use chrono::offset::Local;
use crate::args::{ParsedArgs, ParserError};
use crate::constants::LOCK_DIR;
use crate::crc::CrcMismatchError;
use crate::secret::SecretError;
use std::io::Write;

fn prompt_password() -> String {
    if cfg!(test) {
        let pass = rpassword::prompt_password_stdout("password:").unwrap();
        print!("\x1b[1A\x1b[2K");
        std::io::stdout().flush().unwrap();
        pass
    } else {
        "1234".to_owned()
    }
}

fn main_app(args_list: &[String]) -> Result<String, VaultCliError> {
    let args = ParsedArgs::from_args(args_list);

    match args.get_index(1) {
        // "login" => { /* login */ }
        Some("fget") => {
            let path = args.expect_index(2, "secret_path")?;
            args.expect_none_except(3, &[])?;
            let password = prompt_password();
            let matched = secret::get_secret_files(path, &password)?;
            Ok(matched.join("\n"))
        }
        Some("fset") => {
            let path = args.expect_index(2, "secret_path")?;
            args.expect_none_except(3, &[])?;
            let password = prompt_password();
            let matched = secret::set_secret_files(path, &password)?;
            Ok(matched.join("\n"))
        }
        Some("fclr") => {
            let path = args.expect_index(2, "secret_path")?;
            args.expect_none_except(3, &[])?;
            let matched = secret::clear_secret_files(path);
            Ok(matched.join("\n"))
        }
        Some("get") => {
            let path = args.expect_index(2, "secret_path")?;
            args.expect_none_except(3, &[])?;
            let password = prompt_password();
            let contents = secret::get_secret(path, &password)?;
            Ok(contents)
        }
        Some("set") => {
            let path = args.expect_index(2, "secret_path")?;
            let contents_raw = args.expect_index(3, "contents")?;
            args.expect_none_except(4, &[])?;
            let password = prompt_password();
            let contents = &contents_raw.replace("\\n", "\n");
            secret::set_secret(path, contents, &password)?;
            Ok("ok".to_owned())
        }
        Some("rm") => {
            let path = args.expect_index(2, "secret_path")?;
            args.expect_none_except(3, &[])?;
            let password = prompt_password();
            secret::remove_secret(path, &password)?;
            Ok("ok".to_owned())
        }
        Some("ls") => {
            let pattern = args.get_index(2).unwrap_or("");
            args.expect_none_except(3, &[])?;
            let password = prompt_password();
            let matched = secret::list_secret_paths(pattern, &password)?;
            Ok(matched.join("\n"))
        }
        Some("crc") => {
            args.expect_none_except(2, &["force-update"])?;
            if args.get_value("force-update").is_some() {
                crc::update_crc_all(LOCK_DIR)
            } else {
                crc::check_crc_all(LOCK_DIR)?
            }
            Ok("ok".to_owned())
        }
        Some("zip") => {
            args.expect_none_except(2, &[])?;
            let datestamp = Local::now().format("%Y%m%d");
            let zip_name = format!("vault-{}.zip", datestamp);
            let zip_entries = &[LOCK_DIR, "vault"];
            let matched = zip::zip_files(&zip_name, zip_entries).unwrap();
            Ok(matched.join("\n"))
        }
        Some(_) => {
            Err(ParserError::invalid_value("command").into())
        }
        None => {
            if args.get_value("version").is_some() {
                // TODO: print version
                Ok("0.3".to_owned())
            } else if args.get_value("help").is_some() {
                // TODO: print help
                Ok("help is here".to_owned())
            } else {
                Err(ParserError::missing_value("command").into())
            }
        }
    }
}

fn main() -> () {
    let args_list = std::env::args().collect::<Vec<_>>();
    match main_app(&args_list) {
        Ok(stdout) => if !stdout.is_empty() {
            println!("{}", stdout);
        }
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

pub type VaultCliError = String;

impl From<ParserError> for VaultCliError {
    fn from(error: ParserError) -> Self {
        (match error {
            ParserError::TooManyIndexed =>
                "too many parameters were passed".to_owned(),
            ParserError::InvalidKey { key } =>
                format!("parameter '{}' was not expected, but found", key),
            ParserError::MissingValue { key } =>
                format!("parameter '{}' was expected, but not found", key),
            ParserError::InvalidValue { key } =>
                format!("parameter '{}' provided was invalid", key),
        }) + "\npass '--help' to obtain usage instructions"
    }
}

impl From<CrcMismatchError> for VaultCliError {
    fn from(error: CrcMismatchError) -> Self {
        format!("crc mismatch found for file path '{}'", error.file_path)
            + "\ncheck backups for last correct version"
    }
}

impl From<SecretError> for VaultCliError {
    fn from(error: SecretError) -> Self {
        match error {
            SecretError::CrcMismatch { file_path } =>
                format!("crc mismatch found for file path '{}'", file_path)
                    + "\ncheck backups for last correct version",
            SecretError::IncorrectPassword =>
                "password provided was incorrect".to_owned(),
            SecretError::NonExistentPath =>
                "given secret path does not exist".to_owned(),
        }
    }
}

#[cfg(test)]
mod test {

}
