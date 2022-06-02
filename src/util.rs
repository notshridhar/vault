use std::path::Path;

pub trait VecExt<T> {
    fn into_sorted(self) -> Vec<T>;
}

impl<T: Ord> VecExt<T> for Vec<T> {
    fn into_sorted(mut self) -> Vec<T> {
        self.sort();
        self
    }
}

pub trait PathExt {
    fn to_path_str(&self) -> &str;
    fn to_filename_str(&self) -> &str;
}

impl<P: AsRef<Path>> PathExt for P {
    fn to_path_str(&self) -> &str {
        self.as_ref().to_str().unwrap()
    }

    fn to_filename_str(&self) -> &str {
        self.as_ref().file_name().unwrap().to_str().unwrap()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use super::{PathExt, VecExt};

    #[test]
    fn should_sort_vec_integers() {
        assert_eq!([2, 1, 3].to_vec().into_sorted(), [1, 2, 3]);
    }

    #[test]
    fn should_get_unicode_str_for_path() {
        let path = Path::new("test").join("path.txt");
        assert_eq!(path.to_path_str(), "test/path.txt");
    }

    #[test]
    fn should_get_filename_str_for_path() {
        let path = Path::new("test").join("path.txt");
        assert_eq!(path.to_filename_str(), "path.txt");
    }
}
