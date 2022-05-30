use crate::util::PathExt;
use crc::{Crc, CRC_32_ISCSI};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

fn compute_crc<P: AsRef<Path>>(path: P) -> io::Result<u32> {
    let file_content = fs::read(path)?;
    Ok(CRC32.checksum(&file_content))
}

fn compute_crc_all<P: AsRef<Path>>(root_dir: P) -> HashMap<String, u32> {
    if let Ok(dir_entries) = fs::read_dir(root_dir) {
        dir_entries.fold(HashMap::new(), |mut accum, entry| {
            let file_entry = entry.unwrap();
            let file_name = file_entry.file_name();
            if file_name != "index.crc" {
                let file_content = fs::read(file_entry.path()).unwrap();
                let checksum = CRC32.checksum(&file_content);
                accum.insert(file_name.into_string().unwrap(), checksum);
            }
            accum
        })
    } else {
        HashMap::new()
    }
}

fn read_crc_file<P: AsRef<Path>>(root_dir: P) -> HashMap<String, u32> {
    match fs::read_to_string(root_dir.as_ref().join("index.crc")) {
        Ok(contents) => serde_json::from_str(&contents).unwrap(),
        Err(_) => HashMap::new(),
    }
}

fn write_crc_file<P: AsRef<Path>>(
    crc_map: &HashMap<String, u32>, root_dir: P
) -> () {
    let crc_file_path = root_dir.as_ref().join("index.crc");
    let contents = serde_json::to_string(crc_map).unwrap();
    fs::create_dir_all(root_dir).unwrap();
    fs::write(crc_file_path, contents).unwrap()
}

pub fn check_crc<P: AsRef<Path>, Q: AsRef<Path>>(
    path: P, root_dir: Q
) -> Result<u32, CrcMismatchError> {
    let stored_crc_all = read_crc_file(root_dir);
    match compute_crc(&path) {
        Ok(computed_crc) => match stored_crc_all.get(path.to_unicode_str()) {
            Some(stored_crc) => match stored_crc == &computed_crc {
                true => Ok(computed_crc),
                false => Err(CrcMismatchError::new(path.to_filename_str())),
            }
            None => Err(CrcMismatchError::new(path.to_filename_str())),
        }
        Err(_) => Err(CrcMismatchError::new(path.to_filename_str())),
    }
}

pub fn update_crc<P: AsRef<Path>, Q: AsRef<Path>>(
    path: P, root_dir: Q
) -> () {
    let mut stored_crc = read_crc_file(&root_dir);
    match compute_crc(&path) {
        Ok(crc) => stored_crc.insert(path.to_unicode_str().to_owned(), crc),
        Err(_) => stored_crc.remove(path.to_unicode_str()),
    };
    write_crc_file(&stored_crc, root_dir);
}

pub fn check_crc_all<P: AsRef<Path>>(
    root_dir: P
) -> Result<HashMap<String, u32>, CrcMismatchError> {
    let stored_crc = read_crc_file(&root_dir);
    let computed_crc = compute_crc_all(root_dir);

    let added_errors = computed_crc.keys().filter_map(|computed_key| {
        match stored_crc.contains_key(computed_key) {
            true => None,
            false => Some(CrcMismatchError::new(computed_key)),
        }
    });

    let diff_errors = stored_crc.keys().filter_map(|stored_key| {
        let stored_value = stored_crc.get(stored_key).unwrap();
        match computed_crc.get(stored_key) {
            Some(computed_value) if computed_value == stored_value => None,
            _ => Some(CrcMismatchError::new(stored_key)),
        }
    });

    match added_errors.chain(diff_errors).next() {
        Some(err) => Err(err),
        None => Ok(computed_crc),
    }
}

pub fn update_crc_all<P: AsRef<Path>>(root_dir: P) -> () {
    let computed_crc = compute_crc_all(&root_dir);
    write_crc_file(&computed_crc, root_dir)
}

#[derive(Debug, PartialEq)]
pub struct CrcMismatchError {
    pub file_path: String,
}

impl CrcMismatchError {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { file_path: path.into() }
    }
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;
    use super::PathExt;

    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    const CRC_DIR: &'static str = "crc-test";

    #[test]
    fn should_pass_crc_check_when_intact() {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRC_DIR).unwrap_or_default();
        let file_path = Path::new(CRC_DIR).join("path");
        fs::write(&file_path, "first_val").unwrap();
        super::update_crc(&file_path, CRC_DIR);
        super::check_crc(file_path, CRC_DIR).unwrap();
        fs::remove_dir_all(CRC_DIR).unwrap();
        drop(lock);
    }

    #[test]
    fn should_pass_crc_check_all_when_intact() {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRC_DIR).unwrap_or_default();
        let file_path = Path::new(CRC_DIR).join("path");
        fs::write(file_path, "first_val").unwrap();
        super::update_crc_all(CRC_DIR);
        super::check_crc_all(CRC_DIR).unwrap();
        fs::remove_dir_all(CRC_DIR).unwrap();
        drop(lock);
    }

    #[test]
    fn should_not_pass_crc_check_when_corrupt() {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRC_DIR).unwrap_or_default();
        let file_path = Path::new(CRC_DIR).join("path");
        fs::write(&file_path, "first_val").unwrap();
        super::update_crc(&file_path, CRC_DIR);
        fs::write(&file_path, "second_val").unwrap();
        let err = super::check_crc(&file_path, CRC_DIR).unwrap_err();
        let file_name_str = file_path.to_filename_str();
        assert_eq!(err, super::CrcMismatchError::new(file_name_str));
        fs::remove_dir_all(CRC_DIR).unwrap();
        drop(lock);
    }

    #[test]
    fn should_not_pass_crc_check_all_when_corrupt() {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRC_DIR).unwrap_or_default();
        let file_path = Path::new(CRC_DIR).join("path");
        fs::write(&file_path, "first_val").unwrap();
        super::update_crc_all(CRC_DIR);
        fs::write(&file_path, "second_val").unwrap();
        let err = super::check_crc_all(CRC_DIR).unwrap_err();
        let file_name_str = file_path.to_filename_str();
        assert_eq!(err, super::CrcMismatchError::new(file_name_str));
        fs::remove_dir_all(CRC_DIR).unwrap();
        drop(lock);
    }
}
