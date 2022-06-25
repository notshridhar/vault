mod arg;
mod constant;
mod crc;
mod crypto;
mod secret;
mod tui;
mod util;

use chrono::offset::Local;
use crate::arg::{ParsedArgs, ParserError, HelpGenerator};
use crate::constant::LOCK_DIR;
use crate::crc::CrcMismatchError;
use crate::secret::SecretError;
use crate::util::zip::Zipper;
use std::io::{self, Write};
use termion::input::TermRead;

/// Prompts for password in stdin.
/// Clears prompt after the password is entered.
/// In test context, this just returns a default value.
fn prompt_password() -> String {
    if !cfg!(test) {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        stdout.write_all(b"password: ").unwrap();
        stdout.flush().unwrap();
        let pass = stdin
            .read_passwd(&mut stdout)
            .unwrap()
            .unwrap_or_default();
        stdout.write_all(b"\r").unwrap();
        stdout.write_all(" ".repeat(10).as_bytes()).unwrap();
        stdout.write_all(b"\r").unwrap();
        stdout.flush().unwrap();
        pass
    } else {
        "1234".to_owned()
    }
}

/// Gets fully formatted help string.
fn get_help_string() -> String {
    let mut generator = HelpGenerator::new();
    generator.push_section("usage");
    generator.push_line("", "vault [options] command args");
    generator.push_section("commands");
    generator.push_line("tui","
        starts vault in interactive mode
        this is the recommended way of using vault
        -----
    ");
    generator.push_line("get", "
        prints the secret contents at the given path
        usage: get <path>
        -----
    ");
    generator.push_line("set", "
        sets the secret contents at the given path
        creates new path if the path is not found
        replaces existing contents otherwise
        usage: set <path> <contents>
        -----
    ");
    generator.push_line("rm", "
        removes the given path and its contents
        usage: rm <path>
        -----
    ");
    generator.push_line("ls", "
        lists the paths matching the given pattern
        usage: ls <path-pattern>
        -----
    ");
    generator.push_line("fget", "
        decrypts paths matching the given pattern
        also works with non-unicode contents unlike get
        usage: fget <path-pattern>
        -----
    ");
    generator.push_line("fset", "
        encrypts paths matching the given pattern
        also works with non-unicode contents unlike set
        usage: fset <path-pattern>
        -----
    ");
    generator.push_line("fclr", "
        removes unlocked paths matching the given pattern
        does not affect the actual secret path or contents
        usage: fclr <path-pattern>
        -----
    ");
    generator.push_line("crc", "
        checks crc integrity for all paths and contents
        passing '--force-update' updates all checksums
        usage: crc [--force-update]
        -----
    ");
    generator.push_line("zip", "
        packs the encrypted contents for backup
    ");
    generator.push_section("options");
    generator.push_line("--help", "show this help message and exit");
    generator.push_line("--version", "show the current version and exit");
    generator.generate()
}

/// Testable entry point. Except for the interactive `login` command,
/// none of the commands directly modify `stdout` or read from `stdin`.
fn main_app<I>(args: I) -> Result<String, VaultCliError>
where I: IntoIterator<Item = String> {
    let args = ParsedArgs::from_iter(args);
    match args.get_index(1) {
        Some("tui") => {
            tui::start_event_loop_blocking();
            Ok("".to_owned())
        }
        Some("get") => {
            let path = args.expect_index(2, "path")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            let contents = secret::get_secret(path, &password)?;
            Ok(contents)
        }
        Some("set") => {
            let path = args.expect_index(2, "path")?;
            let contents_raw = args.expect_index(3, "contents")?;
            args.expect_no_index_over(3)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            let contents = &contents_raw.replace("\\n", "\n");
            secret::set_secret(path, contents, &password)?;
            Ok("ok".to_owned())
        }
        Some("rm") => {
            let path = args.expect_index(2, "path")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            secret::remove_secret(path, &password)?;
            Ok("ok".to_owned())
        }
        Some("ls") => {
            let pattern = args.expect_index(2, "path-pattern")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            let matched = secret::list_secret_paths(pattern, &password)?;
            Ok(matched.join("\n"))
        }
        Some("fget") => {
            let path = args.expect_index(2, "path-pattern")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            let matched = secret::get_secret_files(path, &password)?;
            Ok(matched.join("\n"))
        }
        Some("fset") => {
            let path = args.expect_index(2, "path-pattern")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let password = prompt_password();
            let matched = secret::set_secret_files(path, &password)?;
            Ok(matched.join("\n"))
        }
        Some("fclr") => {
            let path = args.expect_index(2, "path-pattern")?;
            args.expect_no_index_over(2)?;
            args.expect_no_keys_except(&[])?;
            let matched = secret::clear_secret_files(path);
            Ok(matched.join("\n"))
        }
        Some("crc") => {
            args.expect_no_index_over(1)?;
            args.expect_no_keys_except(&["force-update"])?;
            if args.get_value("force-update").is_some() {
                crc::update_crc_all(LOCK_DIR)
            } else {
                crc::check_crc_all(LOCK_DIR)?
            }
            Ok("ok".to_owned())
        }
        Some("zip") => {
            args.expect_no_index_over(1)?;
            args.expect_no_keys_except(&[])?;
            let datestamp = Local::now().format("%Y%m%d");
            let mut zipper = Zipper::new(format!("vault-{}.zip", datestamp));
            zipper.zip_dir(LOCK_DIR);
            zipper.zip_file("vault");
            let matches = zipper.finish();
            Ok(matches.join("\n"))
        }
        Some(_) => {
            Err(ParserError::invalid_value("command").into())
        }
        None => {
            if args.get_value("version").is_some() {
                Ok("0.3".to_owned())
            } else if args.get_value("help").is_some() {
                Ok(get_help_string())
            } else {
                Err(ParserError::missing_value("command").into())
            }
        }
    }
}

/// Actual entry point.
fn main() {
    match main_app(std::env::args()) {
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
                format!("parameter '{}' was not expected, but was found", key),
            ParserError::MissingValue { key } =>
                format!("parameter '{}' was expected, but was not found", key),
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
