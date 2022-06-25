use std::io::Write;

/// ANSI terminal color
#[derive(Copy, Clone)]
pub enum Color {
    Red = 1,
    Green = 2,
    Yellow = 3,
    Reset = 9,
}

/// Easy cursor control over ANSI terminals.
pub trait TermControl {
    fn write_str<S: AsRef<str>>(&mut self, value: S);
    fn clear_line_to_end(&mut self);
    fn clear_line_to_start(&mut self);
    fn clear_line_full(&mut self);
    fn clear_screen(&mut self);
    fn show_cursor(&mut self);
    fn hide_cursor(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
    fn move_cursor_to_horiz(&mut self, col: u16);
    fn move_cursor_up(&mut self, rows: u16);
    fn move_cursor_down(&mut self, rows: u16);
    fn move_cursor_right(&mut self, cols: u16);
    fn move_cursor_left(&mut self, cols: u16);
    fn use_color_fg(&mut self, color: Color);
    fn use_color_bg(&mut self, color: Color);
    fn apply_space(&mut self, cols: u16);
    fn apply_backspace(&mut self, cols: u16);
    fn draw_box(&mut self, rows: u16, cols: u16);
}

impl<W: Write> TermControl for W {
    #[inline]
    fn write_str<S: AsRef<str>>(&mut self, value: S) {
        self.write_all(value.as_ref().as_bytes()).unwrap();
    }

    #[inline]
    fn clear_line_to_end(&mut self) {
        write!(self, "\x1b[0K").unwrap();
    }

    #[inline]
    fn clear_line_to_start(&mut self) {
        write!(self, "\x1b[1K").unwrap();
    }

    #[inline]
    fn clear_line_full(&mut self) {
        write!(self, "\x1b[2K").unwrap();
    }

    #[inline]
    fn clear_screen(&mut self) {
        write!(self, "\x1b[2J\x1b[3J").unwrap();
    }

    #[inline]
    fn show_cursor(&mut self) {
        write!(self, "\x1b[?25h").unwrap();
    }

    #[inline]
    fn hide_cursor(&mut self) {
        write!(self, "\x1b[?25l").unwrap();
    }

    #[inline]
    fn move_cursor_to(&mut self, row: u16, col: u16) {
        write!(self, "\x1b[{};{}H", row + 1, col + 1).unwrap();
    }

    #[inline]
    fn move_cursor_to_horiz(&mut self, col: u16) {
        write!(self, "\x1b[{}G", col + 1).unwrap();
    }

    #[inline]
    fn move_cursor_up(&mut self, rows: u16) {
        if rows > 0 {
            write!(self, "\x1b[{}A", rows).unwrap();
        }
    }

    #[inline]
    fn move_cursor_down(&mut self, rows: u16) {
        // for some reason, '\x1b[{}B' completely ceases
        // to work sometimes. (observed in mac terminal)
        self.write_str("\n".repeat(rows as usize))
    }

    #[inline]
    fn move_cursor_right(&mut self, cols: u16) {
        if cols > 0 {
            write!(self, "\x1b[{}C", cols).unwrap();
        }
    }

    #[inline]
    fn move_cursor_left(&mut self, cols: u16) {
        if cols > 0 {
            write!(self, "\x1b[{}D", cols).unwrap();
        }
    }

    #[inline]
    fn use_color_fg(&mut self, color: Color) {
        write!(self, "\x1b[3{}m", color as u8).unwrap()
    }

    #[inline]
    fn use_color_bg(&mut self, color: Color) {
        write!(self, "\x1b[4{}m", color as u8).unwrap()
    }

    fn apply_space(&mut self, cols: u16) {
        (0..cols).for_each(|_| self.write_all(&[' ' as u8]).unwrap());
    }

    fn apply_backspace(&mut self, cols: u16) {
        self.move_cursor_left(cols);
        self.write_str(" ".repeat(cols as usize));
        self.move_cursor_left(cols);
    }

    fn draw_box(&mut self, rows: u16, cols: u16) {
        self.write_str("\u{250c}");
        self.write_str("\u{2500}".repeat(cols as usize - 2));
        self.write_str("\u{2510}\n");
        self.move_cursor_left(cols);
        (2..rows).for_each(|_| {
            self.write_str("\u{2502}");
            self.move_cursor_right(cols - 2);
            self.write_str("\u{2502}\n");
            self.move_cursor_left(cols);
        });
        self.write_str("\u{2514}");
        self.write_str("\u{2500}".repeat(cols as usize - 2));
        self.write_str("\u{2518}");
    }    
}
