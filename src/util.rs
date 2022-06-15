use std::io::Write;
use std::path::Path;
use termion::color::Color;

/// Extension trait for `Vec` collection.
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
    #[inline(always)]
    fn into_sorted(mut self) -> Self {
        self.sort();
        self
    }

    #[inline(always)]
    fn push_inplace(mut self, item: T) -> Self {
        self.push(item);
        self
    }

    #[inline(always)]
    fn extend_inplace<I: IntoIterator<Item = T>>(mut self, iter: I) -> Self {
        self.extend(iter);
        self
    }
}

/// Extension trait for `Path`-like structs.
pub trait PathExt {
    /// Yields a [`&str`] slice.
    /// Panics if the path is not valid utf-8.
    fn to_path_str(&self) -> &str;

    /// Returns the final component of the Path.
    /// Panics if the name is not valid utf-8.
    fn to_filename_str(&self) -> &str;
}

impl<P: AsRef<Path>> PathExt for P {
    #[inline(always)]
    fn to_path_str(&self) -> &str {
        self.as_ref().to_str().unwrap()
    }

    #[inline(always)]
    fn to_filename_str(&self) -> &str {
        self.as_ref().file_name().unwrap().to_str().unwrap()
    }
}

/// Extension trait for `Write` trait.
pub trait WriteExt {
    fn write_str<S: AsRef<str>>(&mut self, value: S);
    fn clear_current_line(&mut self);
    fn clear_screen(&mut self);
    fn show_cursor(&mut self);
    fn hide_cursor(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
    fn move_cursor_to_horiz(&mut self, col: u16);
    fn move_cursor_up(&mut self, rows: u16);
    fn move_cursor_down(&mut self, rows: u16);
    fn move_cursor_right(&mut self, cols: u16);
    fn move_cursor_left(&mut self, cols: u16);
    fn use_color_fg<C: Color>(&mut self, color: C);
    fn use_color_bg<C: Color>(&mut self, color: C);
    fn apply_backspace(&mut self, cols: u16);
}

impl<W: Write> WriteExt for W {
    #[inline(always)]
    fn write_str<S: AsRef<str>>(&mut self, value: S) {
        self.write_all(value.as_ref().as_bytes()).unwrap();
    }

    #[inline(always)]
    fn clear_current_line(&mut self) {
        write!(self, "\x1b[2K").unwrap();
    }

    #[inline(always)]
    fn clear_screen(&mut self) {
        write!(self, "\x1b[2J\x1b[3J").unwrap();
    }

    #[inline(always)]
    fn show_cursor(&mut self) {
        write!(self, "{}", termion::cursor::Show).unwrap();
    }

    #[inline(always)]
    fn hide_cursor(&mut self) {
        write!(self, "{}", termion::cursor::Hide).unwrap();
    }

    #[inline(always)]
    fn move_cursor_to(&mut self, row: u16, col: u16) {
        write!(self, "\x1b[{};{}H", row + 1, col + 1).unwrap();
    }

    #[inline(always)]
    fn move_cursor_to_horiz(&mut self, col: u16) {
        write!(self, "\x1b[{}G", col + 1).unwrap();
    }

    #[inline(always)]
    fn move_cursor_up(&mut self, rows: u16) {
        if rows > 0 {
            write!(self, "\x1b[{}A", rows).unwrap();
        }
    }

    #[inline(always)]
    fn move_cursor_down(&mut self, rows: u16) {
        if rows > 0 {
            write!(self, "\x1b[{}B", rows).unwrap();
        }
    }

    #[inline(always)]
    fn move_cursor_right(&mut self, cols: u16) {
        if cols > 0 {
            write!(self, "\x1b[{}C", cols).unwrap();
        }
    }

    #[inline(always)]
    fn move_cursor_left(&mut self, cols: u16) {
        if cols > 0 {
            write!(self, "\x1b[{}D", cols).unwrap();
        }
    }

    #[inline(always)]
    fn use_color_fg<C: Color>(&mut self, color: C) {
        write!(self, "{}", termion::color::Fg(color)).unwrap()
    }

    #[inline(always)]
    fn use_color_bg<C: Color>(&mut self, color: C) {
        write!(self, "{}", termion::color::Bg(color)).unwrap()
    }

    #[inline(always)]
    fn apply_backspace(&mut self, cols: u16) {
        self.move_cursor_left(cols);
        self.write_str(" ".repeat(cols as usize));
        self.move_cursor_left(cols);
    }
}

/// User interface extension trait for `Write` trait.
pub trait WriteUiExt {
    fn draw_box(&mut self, width: u16, height: u16);
    fn draw_window_title(&mut self, title: &str);
}

impl<W: Write> WriteUiExt for W {
    fn draw_box(&mut self, width: u16, height: u16) {
        self.write_str("┌");
        self.write_str("─".repeat(width as usize - 2));
        self.write_str("┐\n");
        self.move_cursor_left(width);
        (2..height).for_each(|_| {
            self.write_str("│");
            self.move_cursor_right(width - 2);
            self.write_str("│\n");
            self.move_cursor_left(width);
        });
        self.write_str("└");
        self.write_str("─".repeat(width as usize - 2));
        self.write_str("┘");
        self.move_cursor_up(height);
        self.move_cursor_left(width);
    }

    fn draw_window_title(&mut self, title: &str) {     
        self.move_cursor_right(2);   
        self.write_str(" ");
        self.write_str(title);
        self.write_str(" ");
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
