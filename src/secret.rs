use crate::constants::{LOCK_DIR, UNLOCK_DIR};
use crate::crc::{self, CrcMismatchError};
use crate::crypto;
use crate::glob;
use crate::util::{VecExt, PathExt};
use orion::errors::UnknownCryptoError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Returns the path to the index file.
macro_rules! index_file_path {
    () => {
        Path::new(LOCK_DIR).join("index.vlt")
    }
}

/// Returns the path to the encrypted file with the given index.
macro_rules! lock_file_path {
    ($index:expr) => {
        Path::new(LOCK_DIR).join(format!("{:0>3}.vlt", $index))
    }
}

/// Returns the path to the decrypted file with the given relative path.
macro_rules! unlock_file_path {
    ($path:expr) => {
        Path::new(UNLOCK_DIR).join($path)
    }
}

/// Hashmap that maps secret paths to numbers.
/// These numbers can be used to identify encrypted files.
type IndexMap = HashMap<String, u16>;

type SecretResult<T> = Result<T, SecretError>;

/// Reads the contents of the index file into a hashmap.
/// - If the file does not exist, returns an empty map.
/// - If the password is incorrect, returns `IncorrectPassword`.
fn read_index_file<P>(path: P, pass: &str) -> SecretResult<IndexMap>
where P: AsRef<Path> {
    crypto::read_file_de(path, pass)
        .map(|val| val.unwrap_or(HashMap::new()))
        .map_err(|err| err.into())
}

/// Writes out the hashmap into the index file.
fn write_index_file<P>(path: P, map: &IndexMap, pass: &str) -> SecretResult<()>
where P: AsRef<Path> {
    crypto::write_file_ser(path, map, pass)
        .map_err(|err| err.into())
}

/// Reserves an index for a path in the given hashmap.
/// The least available index is reserved.
fn reserve_index(map: &mut IndexMap, path: &str) -> u16 {
    let new_index = 1 + map.values()
        .map(|value| value.to_owned())
        .collect::<Vec<_>>()
        .into_sorted()
        .into_iter()
        .fold(0, |accum, val| accum + (accum + 1 == val) as u16);
    map.insert(path.to_owned(), new_index);
    new_index
}

/// Gets the index for the given path, or reserves it otherwise.
fn get_or_reserve_index(map: &mut IndexMap, path: &str) -> u16 {
    match map.get(path) {
        Some(value) => value.to_owned(),
        None => reserve_index(map, path),
    }
}

/// Returns the secret contents for the given path.
/// - If the path does not exist, returns `NonExistentPath`.
/// - If the password is incorrect, returns `IncorrectPassword`.
/// - If the checksum verification fails, returns `CrcMismatch`.
pub fn get_secret(path: &str, pass: &str) -> SecretResult<String> {
    let index_map = read_index_file(index_file_path!(), pass)?;
    if let Some(enc_index) = index_map.get(path) {
        let enc_path = lock_file_path!(enc_index);
        crc::check_crc(&enc_path, LOCK_DIR)?;
        let contents = crypto::read_file_str(enc_path, pass)?;
        Ok(contents.unwrap_or("<byte>".to_owned()))
    } else {
        Err(SecretError::NonExistentPath)
    }
}

/// Sets the secret contents for the given path.
/// - If the password is incorrect, returns `IncorrectPassword`.
pub fn set_secret(path: &str, contents: &str, pass: &str) -> SecretResult<()> {
    let index_path = index_file_path!();
    let mut index_map = read_index_file(&index_path, pass)?;
    let enc_index = get_or_reserve_index(&mut index_map, path);
    let enc_path = lock_file_path!(enc_index);
    write_index_file(index_path, &index_map, pass)?;
    crypto::write_file_str(&enc_path, contents, pass)?;
    crc::update_crc(enc_path, LOCK_DIR);
    Ok(())
}

/// Removes the secret contents from the given path.
/// - If the path does not exist, returns `NonExistentPath`.
/// - If the password is incorrect, returns `IncorrectPassword`.
pub fn remove_secret(path: &str, pass: &str) -> SecretResult<()> {
    let index_path = index_file_path!();
    let mut index_map = read_index_file(&index_path, pass)?;
    if let Some(enc_index) = index_map.get(path) {
        let enc_path = lock_file_path!(enc_index);
        index_map.remove(path);
        write_index_file(index_path, &index_map, pass)?;
        fs::remove_file(&enc_path).unwrap();
        crc::update_crc(enc_path, LOCK_DIR);
        Ok(())
    } else {
        Err(SecretError::NonExistentPath)
    }
}

/// Lists all the secret paths matching the given pattern.
/// - If the password is incorrect, returns `IncorrectPassword`.
pub fn list_secret_paths(pat: &str, pass: &str) -> SecretResult<Vec<String>> {
    let index_map = read_index_file(index_file_path!(), pass)?;
    Ok(glob::filter_matching(index_map.into_keys(), pat))
}

/// Decrypts the secret contents of paths matching the given pattern and
/// writes them into corresponding files in `unlock` directory.
/// - If the password is incorrect, returns `IncorrectPassword`.
/// - If the checksum verification fails, returns `CrcMismatch`.
pub fn get_secret_files(pat: &str, pass: &str) -> SecretResult<Vec<String>> {
    let index_map = read_index_file(index_file_path!(), pass)?;
    let matched_paths = glob::filter_matching(index_map.keys(), pat);
    Result::from_iter(matched_paths.into_iter().map(|secret_path| {
        let enc_index = index_map.get(&secret_path).unwrap();
        let enc_path = lock_file_path!(enc_index);
        let dec_path = unlock_file_path!(&secret_path);
        crc::check_crc(&enc_path, LOCK_DIR)?;
        crypto::decrypt_file(enc_path, dec_path, pass)?;
        Ok(secret_path.to_owned())
    }))
}

/// Encrypts the contents of paths matching the given pattern and
/// writes them into corresponding secret files in `lock` directory.
/// - If the password is incorrect, returns `IncorrectPassword`.
pub fn set_secret_files(pat: &str, pass: &str) -> SecretResult<Vec<String>> {
    let index_path = index_file_path!();
    let mut index_map = read_index_file(&index_path, pass)?;
    let matched_paths = glob::get_matching_files(pat, UNLOCK_DIR);
    Result::from_iter(matched_paths.into_iter().map(|pathbuf| {
        let path_str = pathbuf.to_path_str();
        let enc_index = get_or_reserve_index(&mut index_map, path_str);
        let enc_path = lock_file_path!(enc_index);
        let dec_path = unlock_file_path!(path_str);
        write_index_file(&index_path, &index_map, pass)?;
        crypto::encrypt_file(dec_path, &enc_path, pass)?;
        crc::update_crc(enc_path, LOCK_DIR);
        Ok(path_str.to_owned())
    }))
}

/// Removes all files matching the given pattern in the `unlock` directory.
/// Using this is recommended to clean up decrypted files after their usage.
pub fn clear_secret_files(pat: &str) -> Vec<String> {
    glob::remove_matching_files(pat, UNLOCK_DIR)
        .into_iter()
        .map(|path| path.to_path_str().to_owned())
        .collect()
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
    use crate::constants::{LOCK_DIR, UNLOCK_DIR};
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use std::fs;
    use std::panic;
    use std::path::Path;
    use std::sync::Mutex;

    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(LOCK_DIR).unwrap();
        fs::create_dir_all(UNLOCK_DIR).unwrap();
        let result = panic::catch_unwind(|| test());
        fs::remove_dir_all(LOCK_DIR).unwrap_or_default();
        fs::remove_dir_all(UNLOCK_DIR).unwrap_or_default();
        drop(lock);
        assert!(result.is_ok())
    }

    #[test]
    fn should_reserve_zero_in_empty_map() {
        let mut map = HashMap::new();
        assert_eq!(super::reserve_index(&mut map, "key"), 1);
    }

    #[test]
    fn should_reserve_mid_in_sparse_map() {
        let mut map = HashMap::from([
            ("key1".to_owned(), 1),
            ("key2".to_owned(), 2),
            ("key3".to_owned(), 4),
        ]);
        assert_eq!(super::reserve_index(&mut map, "key4"), 3);
    }

    #[test]
    fn should_reserve_last_in_full_map() {
        let mut map = HashMap::from([
            ("key1".to_owned(), 1),
            ("key2".to_owned(), 2),
            ("key3".to_owned(), 3),
        ]);
        assert_eq!(super::reserve_index(&mut map, "key4"), 4);
    }

    #[test]
    fn should_set_secret() {
        let root_dir = Path::new(LOCK_DIR);
        run_test(|| {
            super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
            assert!(fs::read(root_dir.join("001.vlt")).is_ok());
            assert!(fs::read(root_dir.join("index.vlt")).is_ok());
            assert!(fs::read(root_dir.join("index.crc")).is_ok());
        })
    }

    #[test]
    fn should_get_existent_secret_path() {
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        run_test(|| {
            super::set_secret(test_path, test_val, test_pass).unwrap();
            let found_val = super::get_secret(test_path, test_pass).unwrap();
            assert_eq!(found_val, test_val);
        })
    }

    #[test]
    fn should_not_get_non_existent_secret_path() {
        let (test_path, test_pass) = ("dir1/fil1", "1234");
        let error = Err(super::SecretError::NonExistentPath);
        run_test(|| {
            assert_eq!(super::get_secret(test_path, test_pass), error);
        })
    }

    #[test]
    fn should_not_get_secret_using_incorrect_pass() {
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        let error = Err(super::SecretError::IncorrectPassword);
        run_test(|| {
            super::set_secret(test_path, test_val, test_pass).unwrap();
            assert_eq!(super::get_secret(test_path, "4321"), error);
        })
    }

    #[test]
    fn should_remove_existent_secret_path() {
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        let error = Err(super::SecretError::NonExistentPath);
        run_test(|| {
            super::set_secret(test_path, test_val, test_pass).unwrap();
            super::remove_secret(test_path, test_pass).unwrap();
            assert_eq!(super::get_secret(test_path, test_pass), error);
        })
    }

    #[test]
    fn should_not_remove_non_existent_secret_path() {
        let (test_path, test_pass) = ("dir1/fil1", "1234");
        let error = Err(super::SecretError::NonExistentPath);
        run_test(|| {
            assert_eq!(super::remove_secret(test_path, test_pass), error);
        })
    }

    #[test]
    fn should_list_secret_paths_same_level() {
        let (test_val, pass) = ("contents", "1234");
        run_test(|| {
            super::set_secret("dir1/fil1", test_val, pass).unwrap();
            super::set_secret("dir1/fil2", test_val, pass).unwrap();
            super::set_secret("dir1/sdir/fil3", test_val, pass).unwrap();
            super::set_secret("dir2/fil1", test_val, pass).unwrap();
            let list = super::list_secret_paths("dir1/*", pass).unwrap();
            assert_eq!(list, ["dir1/fil1", "dir1/fil2"]);
        })
    }

    #[test]
    fn should_list_secret_paths_recursive() {
        let (test_val, pass) = ("contents", "1234");
        run_test(|| {
            super::set_secret("dir1/fil1", test_val, pass).unwrap();
            super::set_secret("dir1/sdir/fil3", test_val, pass).unwrap();
            super::set_secret("dir2/fil1", test_val, pass).unwrap();
            let list = super::list_secret_paths("dir1/**", pass).unwrap();
            assert_eq!(list, ["dir1/fil1", "dir1/sdir/fil3"]);
        })
    }

    #[test]
    fn should_set_secret_files() {
        let lock_dir = Path::new(LOCK_DIR);
        let unlock_dir = Path::new(UNLOCK_DIR);
        let (test_path, test_val, pass) = ("path", "contents", "1234");
        run_test(|| {
            fs::write(unlock_dir.join(test_path), test_val).unwrap();
            let matched = super::set_secret_files(test_path, pass).unwrap();
            assert_eq!(matched, [test_path]);
            assert!(fs::read(lock_dir.join("001.vlt")).is_ok());
            assert!(fs::read(lock_dir.join("index.vlt")).is_ok());
            assert!(fs::read(lock_dir.join("index.crc")).is_ok());
        })
    }

    #[test]
    fn should_get_existent_secret_files() {
        let unlock_dir = Path::new(UNLOCK_DIR);
        let (test_path, test_val, pass) = ("path", "contents", "1234");
        let test_path_full = unlock_dir.join(test_path);
        run_test(|| {
            fs::write(&test_path_full, test_val).unwrap();
            assert!(super::set_secret_files(test_path, pass).is_ok());
            fs::remove_file(&test_path_full).unwrap();
            let matched = super::get_secret_files(test_path, pass).unwrap();
            assert_eq!(matched, [test_path]);
            assert_eq!(fs::read_to_string(test_path_full).unwrap(), test_val);
        })
    }

    #[test]
    fn should_not_get_non_existent_secret_files() {
        run_test(|| {
            let matched = super::get_secret_files("path", "1234").unwrap();
            assert_eq!(matched, [] as [&str; 0]);
        })
    }

    #[test]
    fn should_clear_specified_files() {
        let unlock_dir = Path::new(UNLOCK_DIR);
        let (test_path, test_val) = ("path", "contents");
        run_test(|| {
            fs::write(unlock_dir.join(test_path), test_val).unwrap();
            assert_eq!(super::clear_secret_files("**"), [test_path]);
            assert!(fs::read_dir(unlock_dir).is_err());
        })
    }
}
