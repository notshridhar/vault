use std::path::Path;

/// Extension trait for `Vec` collection
pub trait VecExt<T> {
    /// Sorts the collection.
    /// Returns the modified collection.
    fn into_sorted(self) -> Self;
    
    /// Appends an element to the back of a collection.
    /// Returns the modified collection.
    fn push_inplace(self, item: T) -> Self;

    /// Extends a collection with the contents of the iterator.
    /// Returns the modified collection.
    fn extend_inplace<I: IntoIterator<Item = T>>(self, iter: I) -> Self;
}

impl<T: Ord> VecExt<T> for Vec<T> {
    fn into_sorted(mut self) -> Self {
        self.sort();
        self
    }

    fn push_inplace(mut self, item: T) -> Self {
        self.push(item);
        self
    }

    fn extend_inplace<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
        self.extend(iter);
        self
    }
}

/// Extension trait for `Path`-like values
pub trait PathExt {
    /// Yields a [`&str`] slice.
    /// Panics if the path is not valid utf-8.
    fn to_path_str(&self) -> &str;

    /// Returns the final component of the Path.
    /// Panics if the name is not valid utf-8.
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
