use crate::util::PathExt;
use std::fs;
use std::io::Write;
use std::path::Path;
use zip::ZipWriter;
use zip::result::ZipResult;
use zip::write::FileOptions;

pub fn zip_files<P, Q>(path: P, include: &[Q]) -> ZipResult<Vec<String>>
where P: AsRef<Path>, Q: AsRef<Path> {
    let archive = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(archive);
    let mut zipped_paths = Vec::with_capacity(10);
    let options = FileOptions::default();
    for entry in include {
        if let Ok(file_contents) = fs::read(entry) {
            zip.start_file(entry.to_path_str(), options)?;
            zip.write_all(&file_contents)?;
            zipped_paths.push(entry.to_path_str().to_owned());
        } else if let Ok(dir_entries) = fs::read_dir(entry) {
            zip.add_directory(entry.to_path_str(), options)?;
            for dir_entry in dir_entries {
                let file_path = dir_entry.unwrap().path();
                let file_contents = fs::read(&file_path).unwrap();
                zip.start_file(file_path.to_path_str(), options)?;
                zip.write_all(&file_contents)?;
                zipped_paths.push(file_path.to_path_str().to_owned());
            }
        }
    }
    zip.finish()?;
    Ok(zipped_paths)
}

#[cfg(test)]
mod test {
    use crate::util::PathExt;
    use once_cell::sync::Lazy;
    use std::fs;
    use std::panic;
    use std::path::Path;
    use std::sync::Mutex;

    const ZIP_DIR: &'static str = "zip-test-dir";
    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(ZIP_DIR).unwrap();
        let result = panic::catch_unwind(|| test());
        fs::remove_dir_all(ZIP_DIR).unwrap();
        drop(lock);
        assert!(result.is_ok())
    }

    #[test]
    fn should_zip_files() {
        let zip_dir = Path::new(ZIP_DIR);
        let file_path = zip_dir.join("file1");
        let final_file = "final.zip";
        run_test(|| {
            fs::write(&file_path, "contents").unwrap();
            let matched = super::zip_files(final_file, &[zip_dir]).unwrap();
            assert_eq!(matched, [file_path.to_path_str()]);
            assert!(fs::read(final_file).is_ok());
            fs::remove_file(final_file).unwrap();
        })
    }
}
