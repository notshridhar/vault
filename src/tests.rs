use crate::crypto;
use crate::utils;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

type TestResult = Result<(), Box<dyn Error>>;

static FS_RESOURCE: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn setup_files() -> TestResult {
    // unlock
    // +- file1
    // +- dir1
    //    +- file2
    //    +- dir2
    //       +- file3
    fs::remove_dir_all("unlock").unwrap_or_default();
    fs::create_dir_all("unlock/dir1")?;
    fs::create_dir_all("unlock/dir1/dir2")?;
    fs::write("unlock/file1", "contents1")?;
    fs::write("unlock/dir1/file2", "contents2")?;
    fs::write("unlock/dir1/dir2/file3", "contents3")?;
    Ok(())
}

fn clean_files() -> TestResult {
    fs::remove_dir_all("unlock")?;
    for entry_res in fs::read_dir(".")? {
        let entry_path = entry_res?.path();
        if entry_path.is_file() && entry_path.ends_with(".vlt") {
            fs::remove_file(entry_path)?;
        };
    }
    Ok(())
}

#[test]
fn utils_walk_dir() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let found_paths = utils::walk_dir(Path::new("unlock"))?;
    let mut found_files = found_paths
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect::<Vec<_>>();
    found_files.sort();
    assert_eq!(
        found_files,
        [
            "unlock/dir1/dir2/file3",
            "unlock/dir1/file2",
            "unlock/file1",
        ]
    );

    clean_files()
}

#[test]
fn utils_list_files() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let found_paths = utils::list_files(Path::new("unlock"))?;
    let found_files = found_paths
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(found_files, ["unlock/file1"]);

    clean_files()
}

#[test]
fn utils_list_dirs() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let found_paths = utils::list_dirs(Path::new("unlock"))?;
    let found_dirs = found_paths
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(found_dirs, ["unlock/dir1"]);

    clean_files()
}

#[test]
fn utils_get_parent_dir() -> TestResult {
    let test_path = "dir1/dir2/file1";
    assert_eq!(utils::get_parent_dir(test_path), "dir1/dir2");

    let test_path = "file1";
    assert_eq!(utils::get_parent_dir(test_path), "");

    Ok(())
}

#[test]
fn utils_get_matching_files() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let found_files = utils::get_matching_files("unlock/dir1/file2")?;
    assert_eq!(found_files, ["unlock/dir1/file2"]);

    let found_files = utils::get_matching_files("unlock/file2")?;
    assert_eq!(found_files.len(), 0);

    let found_files = utils::get_matching_files("unlock/*")?;
    assert_eq!(found_files, ["unlock/file1"]);

    let found_files = utils::get_matching_files("unlock/fi*")?;
    assert_eq!(found_files, ["unlock/file1"]);

    let mut found_files = utils::get_matching_files("unlock/**")?;
    found_files.sort_unstable();
    assert_eq!(
        found_files,
        [
            "unlock/dir1/dir2/file3",
            "unlock/dir1/file2",
            "unlock/file1",
        ]
    );

    clean_files()
}

#[test]
fn utils_get_matching_dirs() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let found_dirs = utils::get_matching_dirs("unlock/dir1")?;
    assert_eq!(found_dirs, ["unlock/dir1"]);

    let found_dirs = utils::get_matching_dirs("unlock/dir2")?;
    assert_eq!(found_dirs.len(), 0);

    let found_dirs = utils::get_matching_dirs("unlock/dir*")?;
    assert_eq!(found_dirs, ["unlock/dir1"]);

    clean_files()
}

#[test]
fn utils_remove_matching_files() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    utils::remove_matching_files("unlock/dir1/**")?;
    let found_files = utils::get_matching_files("unlock/**")?;
    let found_dirs = utils::get_matching_dirs("unlock/*")?;
    assert_eq!(found_files, ["unlock/file1"]);
    assert_eq!(found_dirs.len(), 0);

    clean_files()
}

#[test]
fn utils_get_matching_keys() -> TestResult {
    let map = HashMap::from([
        ("dir1/path1".to_string(), "001".to_string()),
        ("dir2/path1".to_string(), "002".to_string()),
        ("dir2/path2".to_string(), "003".to_string()),
    ]);

    let keys = utils::get_matching_keys(&map, "dir1/path1");
    assert_eq!(keys, ["dir1/path1"]);

    let keys = utils::get_matching_keys(&map, "dir2/*");
    assert_eq!(keys, ["dir2/path1", "dir2/path2"]);

    let keys = utils::get_matching_keys(&map, "dir3/*");
    assert_eq!(keys.len(), 0);

    let keys = utils::get_matching_keys(&map, "dir**");
    assert_eq!(keys, ["dir1/path1", "dir2/path1", "dir2/path2"]);

    let keys = utils::get_matching_keys(&map, "abc**");
    assert_eq!(keys.len(), 0);

    Ok(())
}

#[test]
fn utils_get_minimum_available_value() -> TestResult {
    let map = HashMap::from([
        ("dir1/path1".to_string(), "001".to_string()),
        ("dir3/path1".to_string(), "004".to_string()),
        ("dir3/path2".to_string(), "003".to_string()),
    ]);

    let min_val = utils::get_minimum_available_value(&map);
    assert_eq!(min_val, 2);

    Ok(())
}

#[test]
fn crypto_kv() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let map = HashMap::from([
        ("dir1/path1".to_string(), "001".to_string()),
        ("dir3/path1".to_string(), "004".to_string()),
        ("dir3/path2".to_string(), "003".to_string()),
    ]);

    let password = "TestPass123";
    let index_path = "unlock/index.vlt";
    crypto::encrypt_kv(&map, index_path, password)?;
    let found_map = crypto::decrypt_kv(index_path, password)?;
    assert_eq!(found_map.get("dir1/path1").unwrap(), "001");
    assert_eq!(found_map.get("dir3/path1").unwrap(), "004");
    assert_eq!(found_map.get("dir3/path2").unwrap(), "003");

    let password = "WrongPass";
    crypto::decrypt_kv(index_path, password).unwrap_err();

    clean_files()
}

#[test]
fn crypto_fs() -> TestResult {
    let _lock = FS_RESOURCE.lock()?;
    setup_files()?;

    let password = "TestPassword123";
    crypto::encrypt_file("unlock/file1", "unlock/file1_enc", password)?;
    crypto::decrypt_file("unlock/file1_enc", "unlock/file1_dec", password)?;
    assert_eq!(fs::read_to_string("unlock/file1_dec")?, "contents1");

    let contents = crypto::decrypt_file_content("unlock/file1_enc", password)?;
    assert_eq!(contents, "contents1");

    let password = "WrongPass";
    crypto::decrypt_file("unlock/file1_enc", "unlock/fail", password).unwrap_err();

    clean_files()
}
