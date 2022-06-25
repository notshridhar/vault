use super::ext::PathExt;
use std::fs;
use std::io::Write;
use std::path::Path;
use zip::ZipWriter;
use zip::write::FileOptions;

pub struct Zipper {
    options: FileOptions,
    inner: ZipWriter<fs::File>,
    matches: Vec<String>,
}

impl Zipper {
    pub fn new<P: AsRef<Path>>(archive_path: P) -> Self {
        let archive = fs::File::create(archive_path).unwrap();
        Self {
            options: FileOptions::default(),
            inner: ZipWriter::new(archive),
            matches: Vec::new(),
        }
    }

    pub fn zip_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        if let Ok(contents) = fs::read(&path) {
            let path_str = path.to_path_str();
            self.inner.start_file(path_str, self.options).unwrap();
            self.inner.write_all(&contents).unwrap();
            self.matches.push(path_str.to_owned());
            true
        } else {
            false
        }
    }

    pub fn zip_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        if let Ok(dir_entries) = fs::read_dir(&path) {
            let dir_path_str = path.to_path_str();
            self.inner.add_directory(dir_path_str, self.options).unwrap();
            for dir_entry in dir_entries {
                let file_path = dir_entry.unwrap().path();
                let file_path_str = file_path.to_path_str();
                let file_contents = fs::read(&file_path).unwrap();
                self.inner.start_file(file_path_str, self.options).unwrap();
                self.inner.write_all(&file_contents).unwrap();
                self.matches.push(file_path_str.to_owned());
            }
            true
        } else {
            false
        }
    }

    pub fn finish(mut self) -> Vec<String> {
        self.inner.finish().unwrap();
        self.matches
    }
}

#[cfg(test)]
mod test {
    use super::super::ext::PathExt;
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
            let mut zipper = super::Zipper::new(final_file);
            assert_eq!(zipper.zip_dir(zip_dir), true);
            let matches = zipper.finish();
            assert_eq!(matches, [file_path.to_path_str()]);
            assert!(fs::read(final_file).is_ok());
            fs::remove_file(final_file).unwrap();
        })
    }
}
