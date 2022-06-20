use std::io::Write;
use termion::color::Color;

/// Extension trait for `Write` trait.
pub trait WriteExt {
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
    fn clear_line_to_end(&mut self) {
        write!(self, "\x1b[0K").unwrap();
    }

    #[inline(always)]
    fn clear_line_to_start(&mut self) {
        write!(self, "\x1b[1K").unwrap();
    }

    #[inline(always)]
    fn clear_line_full(&mut self) {
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
        // for some reason, '\x1b[{}B' completely ceases
        // to work sometimes. (observed in mac terminal)
        self.write_str("\n".repeat(rows as usize))
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
    fn draw_box(&mut self, rows: u16, cols: u16);
    fn draw_window_title(&mut self, title: &str);
}

impl<W: Write> WriteUiExt for W {
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
        self.move_cursor_up(rows);
        self.move_cursor_left(cols);
    }

    fn draw_window_title(&mut self, title: &str) {
        self.move_cursor_right(3);
        self.write_str(" ");
        self.write_str(title);
        self.write_str(" ");
        self.move_cursor_left(title.len() as u16 + 4)
    }
}
