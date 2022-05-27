use crate::util::VecExt;
use crc::{Crc, CRC_32_ISCSI};
use orion::aead;
use orion::errors::UnknownCryptoError;
use serde::Serialize;
use serde_json;
use std::collections::hash_map::HashMap;
use std::collections::hash_set::HashSet;
use std::fs;
use std::io;

pub const LOCK_DIR: &'static str = "vault-lock";
pub const _UNLOCK_DIR: &'static str = "vault-unlock";

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

fn encrypt(
    data: &[u8], password: &str
) -> Result<Vec<u8>, UnknownCryptoError> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::seal(&secret_key, data)
}

fn decrypt(
    data: &[u8], password: &str
) -> Result<Vec<u8>, UnknownCryptoError> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::open(&secret_key, data)
}

fn read_index_file(
    password: &str
) -> Result<HashMap<String, String>, UnknownCryptoError> {
    let index_file = format!("{}/index.vlt", LOCK_DIR);
    if let Ok(contents_enc) = fs::read(&index_file) {
        let contents_dec = decrypt(&contents_enc, password)?;
        let contents = String::from_utf8(contents_dec).unwrap();
        Ok(serde_json::from_str(&contents).unwrap())
    } else {
        Ok(HashMap::new())
    }
}

fn write_index_file(
    index_map: &HashMap<String, String>, password: &str
) -> Result<(), UnknownCryptoError> {
    let index_file = format!("{}/index.vlt", LOCK_DIR);
    let contents = serde_json::to_string(index_map).unwrap();
    let contents_enc = encrypt(contents.as_bytes(), password)?;
    fs::create_dir_all(LOCK_DIR).unwrap();
    fs::write(index_file, contents_enc).unwrap();
    Ok(())
}

fn read_encrypted_file(
    index: u16, password: &str
) -> Result<String, UnknownCryptoError> {
    let file_enc = format!("{}/{:0>3}.vlt", LOCK_DIR, index);
    let contents_enc = fs::read(&file_enc).unwrap();
    let contents_raw = decrypt(&contents_enc, password)?;
    Ok(String::from_utf8(contents_raw).unwrap_or("<bytes>".to_owned()))
}

fn write_encrypted_file(
    index: u16, contents: &str, password: &str
) -> Result<(), UnknownCryptoError> {
    let file_enc = format!("{}/{:0>3}.vlt", LOCK_DIR, index);
    let contents_enc = encrypt(contents.as_bytes(), password)?;
    fs::create_dir_all(LOCK_DIR).unwrap();
    fs::write(file_enc, contents_enc).unwrap();
    Ok(())
}

fn remove_encrypted_file(index: u16) -> () {
    let file_enc = format!("{}/{:0>3}.vlt", LOCK_DIR, index);
    fs::remove_file(file_enc).unwrap()
}

fn compute_crc(file_path: &str) -> io::Result<u32> {
    let file_content = fs::read(file_path)?;
    Ok(CRC32.checksum(&file_content))
}

fn compute_crc_all() -> HashMap<u16, u32> {
    if let Ok(dir_entries) = fs::read_dir(LOCK_DIR) {
        dir_entries.fold(HashMap::new(), |mut accum, entry| {
            let lock_file = entry.unwrap();
            let file_name = lock_file.file_name().into_string().unwrap();
            let file_index_str = file_name.split_once('.').unwrap().0;
            if let Ok(file_index) = file_index_str.parse::<u16>() {
                let file_content = fs::read(lock_file.path()).unwrap();
                let checksum = CRC32.checksum(&file_content);
                accum.insert(file_index, checksum);
            }
            accum
        })
    } else {
        HashMap::new()
    }
}

fn read_crc_file() -> HashMap<u16, u32> {
    let crc_file = format!("{}/index.crc", LOCK_DIR);
    if let Ok(contents) = fs::read_to_string(&crc_file) {
        serde_json::from_str(&contents).unwrap()
    } else {
        HashMap::new()
    }
}

fn write_crc_file(crc_map: &HashMap<u16, u32>) -> () {
    let crc_file = format!("{}/index.crc", LOCK_DIR);
    let contents = serde_json::to_string(crc_map).unwrap();
    fs::create_dir_all(LOCK_DIR).unwrap();
    fs::write(crc_file, contents).unwrap()
}

fn check_crc(file_index: &u16) -> Result<u32, SecretError> {
    let file_path = format!("{}/{:0>3}.vlt", LOCK_DIR, file_index);
    let stored_crc_all = read_crc_file();
    let error = SecretError::CrcMismatch { index: file_index.to_owned() };
    if let Ok(computed_crc) = compute_crc(&file_path) {
        if let Some(stored_crc) = stored_crc_all.get(file_index) {
            if stored_crc == &computed_crc {
                Ok(computed_crc)
            } else {
                Err(error)
            }
        } else {
            Err(error)
        }
    } else {
        Err(error)
    }
}

fn update_crc(file_index: &u16) -> () {
    let file_path = format!("{}/{:0>3}.vlt", LOCK_DIR, file_index);
    let mut stored_crc_all = read_crc_file();
    if let Ok(computed_crc) = compute_crc(&file_path) {
        stored_crc_all.insert(file_index.to_owned(), computed_crc);
    } else {
        stored_crc_all.remove(file_index);
    }
    write_crc_file(&stored_crc_all);
}

fn reserve_index(
    index_map: &mut HashMap<String, String>, secret_path: &str
) -> u16 {
    let new_index = index_map.values()
        .map(|val| val.parse::<u16>().unwrap())
        .collect::<Vec<_>>()
        .into_sorted()
        .into_iter()
        .fold(0, |accum, val| if accum + 1 == val { val } else { accum });
    index_map.insert(secret_path.to_owned(), new_index.to_string());
    new_index
}

pub fn get_secret(
    secret_path: &str, password: &str
) -> Result<SecretInfo, SecretError> {
    let index_map = read_index_file(password)?;
    if let Some(index_value) = index_map.get(secret_path) {
        let file_index = index_value.parse::<u16>().unwrap();
        check_crc(&file_index)?;
        let contents = read_encrypted_file(file_index, password)?;
        Ok(SecretInfo { path: secret_path.to_owned(), contents })
    } else {
        Err(SecretError::NonExistentPath)
    }
}

pub fn set_secret(
    secret_path: &str, contents: &str, password: &str
) -> Result<SecretInfo, SecretError> {
    let mut index_map = read_index_file(password)?;
    let file_index = if let Some(index_value) = index_map.get(secret_path) {
        index_value.parse::<u16>().unwrap()
    } else {
        let file_index = reserve_index(&mut index_map, secret_path);
        write_index_file(&index_map, password)?;
        file_index
    };
    write_encrypted_file(file_index, contents, password)?;
    update_crc(&file_index);
    Ok(SecretInfo {
        path: secret_path.to_owned(),
        contents: contents.to_owned(),
    })
}

pub fn remove_secret(
    secret_path: &str, password: &str
) -> Result<SecretInfo, SecretError> {
    let mut index_map = read_index_file(password)?;
    if let Some(index_value) = index_map.get(secret_path) {
        let file_index = index_value.parse::<u16>().unwrap();
        index_map.remove(secret_path).unwrap();
        write_index_file(&index_map, password)?;
        remove_encrypted_file(file_index);
        update_crc(&file_index);
        Ok(SecretInfo {
            path: secret_path.to_owned(),
            contents: String::new()
        })
    } else {
        Err(SecretError::NonExistentPath)
    }
}

pub fn list_secret_paths(
    pattern: &str, password: &str
) -> Result<Vec<SecretInfo>, SecretError> {
    let index_map = read_index_file(password)?;
    let key_set = HashSet::<String>::from_iter(index_map
        .into_keys()
        .filter(|key| key.starts_with(pattern))
        .map(|key| {
            let key_levels = key.matches('/').count();
            let pat_levels = pattern.matches('/').count();
            key.split('/').take(pat_levels + 1).collect::<Vec<_>>().join("/")
                + if key_levels == pat_levels { "" } else { "/" }
        })
    );
    Ok(Vec::from_iter(key_set
        .into_iter()
        .map(|key| SecretInfo {
            path: key,
            contents: String::new()
        })
    ))
}

pub fn list_secret_paths_recursive(
    pattern: &str, password: &str
) -> Result<Vec<SecretInfo>, SecretError> {
    let index_map = read_index_file(password)?;
    Ok(Vec::from_iter(index_map
        .into_keys()
        .filter(|key| key.starts_with(pattern))
        .map(|key| SecretInfo {
            path: key,
            contents: String::new()
        })
    ))
}

pub fn check_crc_all() -> Result<HashMap<u16, u32>, SecretError> {
    let stored_crc = read_crc_file();
    let computed_crc = compute_crc_all();

    let added_errors = computed_crc.keys()
        .filter(|computed_key| !stored_crc.contains_key(computed_key))
        .map(|computed_key| SecretError::CrcMismatch {
            index: computed_key.to_owned(),
        });

    let removed_errors = stored_crc.keys()
        .filter(|stored_key| !computed_crc.contains_key(stored_key))
        .map(|stored_key| SecretError::CrcMismatch {
            index: stored_key.to_owned(),
        });

    let diff_errors = stored_crc.keys()
        .filter(|stored_key| {
            let stored_value = stored_crc.get(stored_key).unwrap();
            computed_crc.get(stored_key)
                .map(|computed_value| computed_value != stored_value)
                .unwrap_or(false)
        })
        .map(|stored_key| SecretError::CrcMismatch {
            index: stored_key.to_owned(),
        });

    match added_errors.chain(removed_errors).chain(diff_errors).next() {
        Some(err) => Err(err),
        None => Ok(computed_crc),
    }
}

pub fn update_crc_all() -> Result<(), SecretError> {
    let computed_crc = compute_crc_all();
    Ok(write_crc_file(&computed_crc))
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SecretInfo {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum SecretError {
    CrcMismatch { index: u16 },
    IncorrectPassword,
    NonExistentPath,
}

impl From<UnknownCryptoError> for SecretError {
    fn from(_error: UnknownCryptoError) -> Self {
        Self::IncorrectPassword
    }
}

#[cfg(test)]
mod test {
    use super::VecExt;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn should_set_secret_and_write_correct_files() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        let entries = super::fs::read_dir(super::LOCK_DIR).unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>()
            .into_sorted();
        assert_eq!(entries, ["000.vlt", "index.crc", "index.vlt"]);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_get_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let found = super::get_secret(test_path, test_pass).unwrap();
        assert_eq!(found.path, test_path);
        assert_eq!(found.contents, test_val);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_get_non_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let error = super::get_secret("dir1/fil2", test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_get_secret_using_incorrect_password() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let error = super::get_secret(test_path, "4321").unwrap_err();
        assert_eq!(error, super::SecretError::IncorrectPassword);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_remove_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_val, test_pass) = ("dir1/fil1", "cont1", "1234");
        super::set_secret(test_path, test_val, test_pass).unwrap();
        let found = super::get_secret(test_path, test_pass).unwrap();
        assert_eq!(found.path, test_path);
        assert_eq!(found.contents, test_val);
        super::remove_secret(test_path, test_pass).unwrap();
        let error = super::get_secret(test_path, test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_not_remove_non_existent_secret_path() {
        let lock = DIR_LOCK.lock().unwrap();
        let (test_path, test_pass) = ("dir1/fil1", "1234");
        let error = super::remove_secret(test_path, test_pass).unwrap_err();
        assert_eq!(error, super::SecretError::NonExistentPath);
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_list_secret_paths() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        super::set_secret("dir1/fil2", "cont2", "1234").unwrap();
        super::set_secret("dir1/sdir/fil3", "cont4", "1234").unwrap();
        super::set_secret("dir2/fil1", "cont3", "1234").unwrap();
        let list = super::list_secret_paths("dir1/", "1234").unwrap();
        assert_eq!(
            list.into_iter().map(|x| x.path).collect::<Vec<_>>().into_sorted(),
            ["dir1/fil1", "dir1/fil2", "dir1/sdir/"]
        );
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }

    #[test]
    fn should_list_secret_paths_recursive() {
        let lock = DIR_LOCK.lock().unwrap();
        super::set_secret("dir1/fil1", "cont1", "1234").unwrap();
        super::set_secret("dir1/fil2", "cont2", "1234").unwrap();
        super::set_secret("dir1/sdir/fil3", "cont4", "1234").unwrap();
        super::set_secret("dir2/fil1", "cont3", "1234").unwrap();
        let ls = super::list_secret_paths_recursive("dir1/", "1234").unwrap();
        assert_eq!(
            ls.into_iter().map(|x| x.path).collect::<Vec<_>>().into_sorted(),
            ["dir1/fil1", "dir1/fil2", "dir1/sdir/fil3"]
        );
        super::fs::remove_dir_all(super::LOCK_DIR).unwrap_or_default();
        drop(lock);
    }
}
