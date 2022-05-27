pub trait VecExt<T> {
    fn into_sorted(self) -> Vec<T>;
}

impl<T: Ord> VecExt<T> for Vec<T> {
    fn into_sorted(mut self) -> Vec<T> {
        self.sort();
        self
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn should_sort_vec_integers() {
        use super::VecExt;
        assert_eq!([2, 1, 3].to_vec().into_sorted(), [1, 2, 3]);
    }
}
