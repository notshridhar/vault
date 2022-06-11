use crate::util::PathExt;
use crc::{Crc, CRC_32_ISCSI};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

type CrcMap = HashMap<String, u32>;
type CrcResult<T> = Result<T, CrcMismatchError>;
const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

/// Computes crc checksum for the given file path.
/// Returns any error while accessing the file.
fn compute_crc<P: AsRef<Path>>(path: P) -> io::Result<u32> {
    let file_content = fs::read(path)?;
    Ok(CRC32.checksum(&file_content))
}

/// Computes crc checksum for all files in given directory (non-recursive).
/// Returns a hashmap mapping file name to its checksum value.
/// - If the given directory does not exist, an empty map is returned.
fn compute_crc_all<P: AsRef<Path>>(root_dir: P) -> CrcMap {
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

/// Reads crc map from an index file in the given directory.
/// Returns a hashmap mapping file name to its checksum value.
/// - If the given directory does not exist, an empty map is returned.
fn read_crc_file<P: AsRef<Path>>(root_dir: P) -> CrcMap {
    match fs::read_to_string(root_dir.as_ref().join("index.crc")) {
        Ok(contents) => serde_json::from_str(&contents).unwrap(),
        Err(_) => HashMap::new(),
    }
}

/// Writes crc map into an index file in the given directory.
/// - If the given directory does not exist, new one is created.
fn write_crc_file<P: AsRef<Path>>(crc_map: &CrcMap, root_dir: P) {
    let crc_file_path = root_dir.as_ref().join("index.crc");
    let contents = serde_json::to_string(crc_map).unwrap();
    fs::create_dir_all(root_dir).unwrap();
    fs::write(crc_file_path, contents).unwrap()
}

/// Compares computed crc checksum of the given path with the stored value.
/// - If the comparison fails, returns `CrcMismatchError`.
pub fn check_crc<P, Q>(path: P, root_dir: Q) -> CrcResult<()>
where P: AsRef<Path>, Q: AsRef<Path> {
    let stored_crc_all = read_crc_file(root_dir);
    match compute_crc(&path) {
        Ok(computed_crc) => match stored_crc_all.get(path.to_path_str()) {
            Some(stored_crc) => match stored_crc == &computed_crc {
                true => Ok(()),
                false => Err(CrcMismatchError::new(path.to_filename_str())),
            }
            None => Err(CrcMismatchError::new(path.to_filename_str())),
        }
        Err(_) => Err(CrcMismatchError::new(path.to_filename_str())),
    }
}

/// Computes crc checksum for the given path and updates the stored value.
pub fn update_crc<P, Q>(path: P, root_dir: Q)
where P: AsRef<Path>, Q: AsRef<Path> {
    let mut stored_crc = read_crc_file(&root_dir);
    match compute_crc(&path) {
        Ok(crc) => stored_crc.insert(path.to_path_str().to_owned(), crc),
        Err(_) => stored_crc.remove(path.to_path_str()),
    };
    write_crc_file(&stored_crc, root_dir);
}

/// Compares computed crc checksum of all files in the the given directory
/// with the corresponding stored values.
/// - If the comparison fails, returns `CrcMismatchError`.
pub fn check_crc_all<P: AsRef<Path>>(root_dir: P) -> CrcResult<()> {
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
        None => Ok(()),
    }
}

/// Computes crc checksum for all the files in the given directory and
/// updates the corresponding stored values.
pub fn update_crc_all<P: AsRef<Path>>(root_dir: P) {
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
    use std::panic;
    use std::path::Path;
    use std::sync::Mutex;

    const CRC_DIR: &'static str = "crc-test-dir";
    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRC_DIR).unwrap();
        let result = panic::catch_unwind(|| test());
        fs::remove_dir_all(CRC_DIR).unwrap();
        drop(lock);
        assert!(result.is_ok())
    }

    #[test]
    fn should_pass_crc_check_when_intact() {
        run_test(|| {
            let file_path = Path::new(CRC_DIR).join("path");
            fs::write(&file_path, "first_val").unwrap();
            super::update_crc(&file_path, CRC_DIR);
            assert_eq!(super::check_crc(file_path, CRC_DIR), Ok(()));
        })
    }

    #[test]
    fn should_not_pass_crc_check_when_corrupt() {
        run_test(|| {
            let file_name = "path";
            let file_path = Path::new(CRC_DIR).join(file_name);
            fs::write(&file_path, "first_val").unwrap();
            super::update_crc(&file_path, CRC_DIR);
            fs::write(&file_path, "second_val").unwrap();
            let error = Err(super::CrcMismatchError::new(file_name));
            assert_eq!(super::check_crc(&file_path, CRC_DIR), error);
        })
    }

    #[test]
    fn should_pass_crc_check_all_when_intact() {
        run_test(|| {
            let file_path = Path::new(CRC_DIR).join("path");
            fs::write(file_path, "first_val").unwrap();
            super::update_crc_all(CRC_DIR);
            assert_eq!(super::check_crc_all(CRC_DIR), Ok(()));
        })
    }

    #[test]
    fn should_not_pass_crc_check_all_when_corrupt() {
        run_test(|| {
            let file_name = "path";
            let file_path = Path::new(CRC_DIR).join(file_name);
            fs::write(&file_path, "first_val").unwrap();
            super::update_crc_all(CRC_DIR);
            fs::write(file_path, "second_val").unwrap();
            let error = Err(super::CrcMismatchError::new(file_name));
            assert_eq!(super::check_crc_all(CRC_DIR), error);
        })
    }
}
