use crate::util::algo;
use std::cmp;
use super::config::TermConfig;
use super::ansi::TermControl;

pub struct ListWidgetBuilder {
    pub start: (u16, u16),
    pub size: (u16, u16),
    pub title: String,
    pub list: Vec<String>,
    pub selected_index: u16,
    pub viewport_index: u16,
}

impl ListWidgetBuilder {
    pub fn build(self) -> ListWidget {
        let mut widget = ListWidget {
            start: self.start,
            size: self.size,
            title: self.title,
            list: self.list,
            selected_index: self.selected_index,
            viewport_index: self.viewport_index,
        };
        widget.sanitize();
        widget
    }
}

pub struct ListWidget {
    start: (u16, u16),
    size: (u16, u16),
    title: String,
    list: Vec<String>,
    selected_index: u16,
    viewport_index: u16,
}

impl ListWidget {
    const BORDER_X: u16 = 2;
    const BORDER_Y: u16 = 2;

    fn clamp_selected_index(&self, index: u16) -> u16 {
        let total_rows = self.list.len() as u16;
        cmp::min(index, total_rows.saturating_sub(1))
    }

    fn clamp_viewport_index(&self, index: u16) -> u16 {
        let total_rows = self.list.len() as u16;
        let view_rows = self.size.0 - 4;
        let view_min = self.selected_index.saturating_sub(view_rows - 1);
        let view_max = self.selected_index;
        let view_max_abs = total_rows.saturating_sub(view_rows);
        cmp::max(cmp::min(cmp::min(index, view_max), view_max_abs), view_min)
    }

    fn calculate_scroll_state(&self) -> (u16, u16) {
        let i = self.viewport_index;
        let view_rows = self.size.0 - 2 * Self::BORDER_Y;
        let total_rows = self.list.len() as u16;
        algo::calculate_scroll_state(i, view_rows, total_rows)
    }

    fn render_window(&self) {
        let mut out = TermConfig::get_out();
        out.move_cursor_to(self.start.0, self.start.1);
        out.draw_box(self.size.0, self.size.1);
        out.move_cursor_up(self.size.0);
        out.move_cursor_left(self.size.1 - Self::BORDER_X - 1);
        out.write_str(" ");
        out.write_str(&self.title);
        out.write_str(" ");
    }

    fn render_contents(&self, prev_len_arr: &[u8]) {
        let mut out = TermConfig::get_out();
        let view_top = self.start.0 + Self::BORDER_Y;
        let view_left = self.start.1 + 2 * Self::BORDER_X;
        let view_rows = self.size.0 - 2 * Self::BORDER_Y;
        out.move_cursor_to(view_top, view_left);
        (0..view_rows).for_each(|i| {
            let list_index = (self.viewport_index + i) as usize;
            out.move_cursor_to_horiz(view_left);
            let prev_len = prev_len_arr
                .get(i as usize)
                .map(|val| *val as u16)
                .unwrap_or_default();
            if let Some(key) = self.list.get(list_index) {
                out.write_str(key);
                out.apply_space(prev_len.saturating_sub(key.len() as u16));
            } else {
                out.apply_space(prev_len);
            }
            out.move_cursor_down(1);
        });
    }

    fn render_selector(&self, indicator: &str) {
        if self.list.len() > 0 {
            let mut out = TermConfig::get_out();
            let view_top = self.start.0 + Self::BORDER_Y;
            let view_left = self.start.1 + Self::BORDER_X;
            let view_selector = self.selected_index - self.viewport_index;
            out.move_cursor_to(view_top + view_selector, view_left);
            out.write_str(indicator);
        }
    }

    fn render_scroll(&self) {
        let mut out = TermConfig::get_out();
        let view_top = self.start.0 + Self::BORDER_Y;
        let view_right = self.size.1 - Self::BORDER_X - 1;
        let view_rows = self.size.0 - 2 * Self::BORDER_Y;
        let (scroll_size, scroll_pos) = self.calculate_scroll_state();
        out.move_cursor_to(view_top, view_right);
        (0..view_rows).for_each(|i| {
            let in_range = i >= scroll_pos && i < scroll_pos + scroll_size;
            out.write_str(if in_range { "\u{2502}" } else { " " });
            out.move_cursor_down(1);
            out.move_cursor_left(1);
        });
    }

    pub fn get_viewport_index(&self) -> u16 {
        self.viewport_index
    }

    pub fn get_selected_index(&self) -> u16 {
        self.selected_index
    }

    pub fn set_title(&mut self, title: String) {
        let mut out = TermConfig::get_out();
        let common_index = algo::find_max_common_index(&title, &self.title);
        let start_col = self.start.1 + common_index as u16 + 4;
        let extra_cols = self.title.len().saturating_sub(title.len());
        out.move_cursor_to(self.start.0, start_col);
        out.write_str(&title[common_index..]);
        out.write_str(" ");
        out.write_str("\u{2500}".repeat(extra_cols));
        out.flush().unwrap();
        self.title = title;
    }

    pub fn set_list(&mut self, list: Vec<String>) {
        let view_rows = self.size.0 - 4;
        let selected_index = 0;
        let viewport_index = 0;
        self.render_selector(" ");
        let prev_len_arr = (0..view_rows)
            .map(|line_index| self.list
                .get((self.viewport_index + line_index) as usize)
                .map(|key| key.len() as u8)
                .unwrap_or_default()
            )
            .collect::<Vec<_>>();
        self.selected_index = selected_index;
        self.viewport_index = viewport_index;
        self.list = list;
        self.render_contents(&prev_len_arr);
        self.render_scroll();
        self.render_selector(">");
        TermConfig::get_out().flush().unwrap();
    }

    pub fn set_selected_index(&mut self, index: u16) {
        let view_rows = self.size.0 - 4;
        let total_rows = self.list.len() as u16;
        self.render_selector(" ");
        self.selected_index = self.clamp_selected_index(index);
        let viewport_index = self.clamp_viewport_index(self.viewport_index);
        if viewport_index != self.viewport_index {
            let prev_len_arr = (0..view_rows)
                .map(|line_index| self.list
                    .get((self.viewport_index + line_index) as usize)
                    .map(|key| key.len() as u8)
                    .unwrap_or_default()
                )
                .collect::<Vec<_>>();
            self.viewport_index = viewport_index;
            self.render_contents(&prev_len_arr);
            self.render_scroll();
        }
        self.render_selector(">");
        TermConfig::get_out().flush().unwrap();
    }

    pub fn select_prev(&mut self) {
        self.set_selected_index(self.selected_index.saturating_sub(1));
    }

    pub fn select_next(&mut self) {
        self.set_selected_index(self.selected_index + 1);
    }

    pub fn sanitize(&mut self) {
        self.selected_index = self.clamp_selected_index(self.selected_index);
        self.viewport_index = self.clamp_viewport_index(self.viewport_index);
    }

    pub fn render(&self) {
        self.render_window();
        self.render_contents(&[]);
        self.render_selector(">");
        self.render_scroll();
        TermConfig::get_out().flush().unwrap();
    }
}

pub struct TextEditWidgetBuilder {
    pub start: (u16, u16),
    pub size: (u16, u16),
    pub title: String,
    pub lines: Vec<String>,
    pub cursor_pos: (u16, u16),
    pub viewport_pos: (u16, u16),
}

impl TextEditWidgetBuilder {
    pub fn build(self) -> TextEditWidget {
        let mut widget = TextEditWidget {
            start: self.start,
            size: self.size,
            title: self.title,
            lines: self.lines,
            cursor_pos: self.cursor_pos,
            viewport_pos: self.viewport_pos,
        };
        widget.sanitize();
        widget
    }
}

pub struct TextEditWidget {
    start: (u16, u16),
    size: (u16, u16),
    title: String,
    lines: Vec<String>,
    cursor_pos: (u16, u16),
    viewport_pos: (u16, u16),
}

impl TextEditWidget {
    fn clamp_cursor_pos(&self, pos: (u16, u16)) -> (u16, u16) {
        pos
    }

    fn clamp_viewport_pos(&self, pos: (u16, u16)) -> (u16, u16) {
        pos
    }

    pub fn sanitize(&mut self) {
        self.cursor_pos = self.clamp_cursor_pos(self.cursor_pos);
        self.viewport_pos = self.clamp_viewport_pos(self.viewport_pos);
    }
}
