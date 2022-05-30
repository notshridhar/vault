use crate::constants::{LOCK_DIR, UNLOCK_DIR};
use crate::crc::{self, CrcMismatchError};
use crate::crypto;
use crate::glob;
use crate::util::{VecExt, PathExt};
use orion::errors::UnknownCryptoError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

macro_rules! index_file_path {
    () => {
        Path::new(LOCK_DIR).join("index.vlt")
    }
}

macro_rules! lock_file_path {
    ($index:expr) => {
        Path::new(LOCK_DIR).join(format!("{:0>3}.vlt", $index))
    }
}

macro_rules! unlock_file_path {
    ($path:expr) => {
        Path::new(UNLOCK_DIR).join($path)
    }
}

fn reserve_index(
    index_map: &mut HashMap<String, u16>, secret_path: &str
) -> u16 {
    let new_index = index_map.values()
        .map(|value| value.to_owned())
        .collect::<Vec<_>>()
        .into_sorted()
        .into_iter()
        .fold(0, |accum, val| if accum + 1 == val { val } else { accum });
    index_map.insert(secret_path.to_owned(), new_index);
    new_index
}

pub fn get_secret(
    secret_path: &str, password: &str
) -> Result<String, SecretError> {
    let index_path = index_file_path!();
    let index_map = crypto::deserialize_from_file(index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    if let Some(enc_index) = index_map.get(secret_path) {
        let enc_path = lock_file_path!(enc_index);
        crc::check_crc(&enc_path, LOCK_DIR)?;
        let contents = crypto::read_string_from_file(enc_path, password)?;
        Ok(contents.unwrap_or("<byte>".to_owned()))
    } else {
        Err(SecretError::NonExistentPath)
    }
}

pub fn set_secret(
    secret_path: &str, contents: &str, password: &str
) -> Result<String, SecretError> {
    let index_path = index_file_path!();
    let mut index_map = crypto::deserialize_from_file(&index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    let enc_index = match index_map.get(secret_path) {
        Some(value) => value.to_owned(),
        None => reserve_index(&mut index_map, secret_path),
    };
    let enc_path = lock_file_path!(enc_index);
    crypto::serialize_to_file(&index_path, index_map, password)?;
    crypto::write_str_to_file(&enc_path, contents, password)?;
    crc::update_crc(&enc_path, LOCK_DIR);
    Ok(contents.to_owned())
}

pub fn remove_secret(
    secret_path: &str, password: &str
) -> Result<(), SecretError> {
    let index_path = index_file_path!();
    let mut index_map = crypto::deserialize_from_file(&index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    if let Some(enc_index) = index_map.get(secret_path) {
        let enc_path = lock_file_path!(enc_index);
        index_map.remove(secret_path);
        crypto::serialize_to_file(&index_path, index_map, password)?;
        fs::remove_file(&enc_path).unwrap();
        crc::update_crc(enc_path, LOCK_DIR);
        Ok(())
    } else {
        Err(SecretError::NonExistentPath)
    }
}

pub fn list_secret_paths(
    secret_path_pattern: &str, password: &str
) -> Result<Vec<String>, SecretError> {
    let index_path = index_file_path!();
    let index_map = crypto::deserialize_from_file(index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    Ok(glob::filter_matching(index_map.into_keys(), secret_path_pattern))
}

pub fn get_secret_files(
    secret_path_pattern: &str, password: &str
) -> Result<Vec<String>, SecretError> {
    let index_path = index_file_path!();
    let index_map = crypto::deserialize_from_file(index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    let pattern = secret_path_pattern;
    let matched_paths = glob::filter_matching(index_map.keys(), pattern);
    Result::from_iter(matched_paths.into_iter().map(|secret_path| {
        let enc_index = index_map.get(&secret_path).unwrap();
        let enc_path = lock_file_path!(enc_index);
        let dec_path = unlock_file_path!(&secret_path);
        crc::check_crc(&enc_path, LOCK_DIR)?;
        crypto::decrypt_file(enc_path, dec_path, password)?;
        Ok(secret_path.to_owned())
    }))
}

pub fn set_secret_files(
    secret_path_pattern: &str, password: &str
) -> Result<Vec<String>, SecretError> {
    let index_path = index_file_path!();
    let mut index_map = crypto::deserialize_from_file(&index_path, password)?
        .unwrap_or(HashMap::<String, u16>::new());
    let pattern = secret_path_pattern;
    let matched_paths = glob::get_matching_files(pattern, UNLOCK_DIR);
    Result::from_iter(matched_paths.into_iter().map(|secret_pathbuf| {
        let path_str = secret_pathbuf.to_unicode_str();
        let enc_index = match index_map.get(path_str) {
            Some(value) => value.to_owned(),
            None => reserve_index(&mut index_map, path_str),
        };
        let enc_path = lock_file_path!(enc_index);
        let dec_path = unlock_file_path!(path_str);
        crypto::serialize_to_file(&index_path, &index_map, password)?;
        crypto::encrypt_file(&dec_path, &enc_path, password)?;
        crc::update_crc(enc_path, LOCK_DIR);
        Ok(path_str.to_owned())
    }))
}

pub fn clear_secret_files(
    secret_path_pattern: &str
) -> Result<Vec<String>, SecretError> {
    let matched = glob::remove_matching_files(secret_path_pattern, UNLOCK_DIR)
        .into_iter()
        .map(|path| path.to_unicode_str().to_owned())
        .collect();
    Ok(matched)
}

#[derive(Debug, PartialEq)]
pub struct SecretInfo {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, PartialEq)]
pub enum SecretError {
    CrcMismatch { file_path: String },
    IncorrectPassword,
    NonExistentPath,
}

impl From<CrcMismatchError> for SecretError {
    fn from(error: CrcMismatchError) -> Self {
        Self::CrcMismatch { file_path: error.file_path }
    }
}

impl From<UnknownCryptoError> for SecretError {
    fn from(_error: UnknownCryptoError) -> Self {
        Self::IncorrectPassword
    }
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;
    use std::fs;
    use std::sync::Mutex;
    use super::{VecExt, LOCK_DIR};

    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn should_set_secret_and_write_correct_files() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        let entries = fs::read_dir(LOCK_DIR).unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>()
            .into_sorted();
        assert_eq!(entries, ["000.vlt", "index.crc", "index.vlt"]);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_get_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let contents = super::get_secret(test_path, test_pass).unwrap();
        assert_eq!(contents, test_val);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_get_non_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let error = super::get_secret("dir1/fil2", test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_get_secret_using_incorrect_password() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let error = super::get_secret(test_path, "4321").unwrap_err();
        assert_eq!(error, super::SecretError::IncorrectPassword);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_remove_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let contents = super::get_secret(test_path, test_pass).unwrap();
        assert_eq!(contents, test_val);
        super::remove_secret(test_path, test_pass).unwrap();
        let error = super::get_secret(test_path, test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_remove_non_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_pass) = ("dir1/fil1", "1234");
        let error = super::remove_secret(test_path, test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_list_secret_paths() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        super::set_secret("dir1/fil2", "cont2", "1234").unwrap();
        super::set_secret("dir1/sdir/fil3", "cont4", "1234").unwrap();
        super::set_secret("dir2/fil1", "cont3", "1234").unwrap();
        let list = super::list_secret_paths("dir1/*", "1234").unwrap()
            .into_sorted();
        assert_eq!(list, ["dir1/fil1", "dir1/fil2", "dir1/sdir/"]);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_list_secret_paths_recursive() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        super::set_secret("dir1/fil2", "cont2", "1234").unwrap();
        super::set_secret("dir1/sdir/fil3", "cont4", "1234").unwrap();
        super::set_secret("dir2/fil1", "cont3", "1234").unwrap();
        let list = super::list_secret_paths("dir1/**", "1234").unwrap()
            .into_sorted();
        assert_eq!(list, ["dir1/fil1", "dir1/fil2", "dir1/sdir/fil3"]);
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        drop(lock);
    }
}
