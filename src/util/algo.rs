use std::cmp;

pub fn calculate_scroll_state(i: u16, view: u16, total: u16) -> (u16, u16) {
    if total > view { 
        let scroll_size = cmp::max((view * view) / total, 1);
        let view_max = total - view;
        let numerator = i * (view - scroll_size);
        let scroll_pos = (2 * numerator + view_max) / (2 * view_max);
        (scroll_size, scroll_pos)
    } else {
        (0, 0)
    }
}

pub fn find_max_common_index(a: &str, b: &str) -> usize {
    let (a_bytes, b_bytes) = (a.as_bytes(), b.as_bytes());
    let max_common_index = cmp::min(a.len(), b.len());
    (0..max_common_index)
        .find(|i| a_bytes[*i] != b_bytes[*i])
        .unwrap_or(max_common_index)
}

#[cfg(test)]
mod test {
    #[test]
    fn should_calculate_correct_scroll_size() {
        for (view, total, expected, message) in [
            (10, 5, 0, "zero_scroll_full_view"),
            (10, 10, 0, "zero_scroll_just_full_view"),
            (10, 20, 5, "half_size_scroll"),
            (10, 50, 2, "small_size_scroll"),
            (10, 1000, 1, "min_size_scroll"),
        ] {
            let calculated = super::calculate_scroll_state(0, view, total);
            assert_eq!(calculated.0, expected, "{}", message);
        }
    }

    #[test]
    fn should_calculate_correct_scroll_pos() {
        for (i, view, total, expected, message) in [
            (1, 10, 5, 0, "zero_scroll_full_view"),
            (2, 10, 10, 0, "zero_scroll_just_full_view"),
            (0, 10, 20, 0, "zero_scroll_partial_view"),
            (3, 10, 20, 2, "some_scroll_partial_view"),
            (10, 10, 20, 5, "max_scroll_partial_view"),
            (0, 10, 1000, 0, "zero_scroll_small_view"),
            (100, 10, 1000, 1, "some_scroll_small_view"),
            (955, 10, 1000, 9, "max_scroll_small_view"),
        ] {
            let calculated = super::calculate_scroll_state(i, view, total);
            assert_eq!(calculated.1, expected, "{}", message);
        }
    }

    #[test]
    fn should_find_max_common_index() {
        for (a, b, expected) in [
            ("rabbit", "rabbit", 6),
            ("fool", "barcode", 0),
            ("garry", "garfield", 3),
            ("football", "foo", 3),
        ] {
            let found = super::find_max_common_index(a, b);
            assert_eq!(found, expected, "({}, {})", a, b);
        }
    }
}
