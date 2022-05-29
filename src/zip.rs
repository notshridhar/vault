use std::fs;
use std::io::Write;
use zip::ZipWriter;
use zip::result::ZipResult;
use zip::write::FileOptions;

pub fn zip_dirs(
    archive_name: &str, include: &[&str]
) -> ZipResult<Vec<String>> {
    let archive = fs::File::create(archive_name).unwrap();
    let mut zip = ZipWriter::new(archive);
    let mut zipped_paths = Vec::with_capacity(10);
    let options = FileOptions::default();
    for entry in include {
        if let Ok(file_contents) = fs::read(entry) {
            zip.start_file(entry.to_owned(), options)?;
            zip.write_all(&file_contents)?;
            zipped_paths.push(entry.to_string());
        } else if let Ok(dir_entries) = fs::read_dir(entry) {
            zip.add_directory(entry.to_owned(), options)?;
            for dir_entry in dir_entries {
                let file_path = dir_entry.unwrap().path();
                let file_contents = fs::read(&file_path).unwrap();
                zip.start_file(file_path.to_string_lossy(), options)?;
                zip.write_all(&file_contents)?;
                zipped_paths.push(file_path.to_string_lossy().to_string());
            }
        }
    }
    zip.finish()?;
    Ok(zipped_paths)
}
