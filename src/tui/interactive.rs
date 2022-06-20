use crate::glob;
use crate::secret::{self, IndexMap};
use crate::term::{WriteExt, WriteUiExt};
use crate::sync::{FakeLock, FakeLockGuard};
use once_cell::sync::Lazy;
use std::cmp;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Write, BufWriter};
use std::sync::atomic::{AtomicU16, Ordering};
use termion::event::{Key, Event};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

type DynWrite = Box<dyn Write + Send + Sync>;
type DynEvents = Box<dyn Iterator<Item = Event> + Send + Sync>;

static TERM_CONF: Lazy<TermConfig> = Lazy::new(TermConfig::new);
static APP_CACHE: Lazy<AppCache> = Lazy::new(AppCache::new);

struct ListWidget {
    size: (u16, u16),
    window_title: String,
}

impl ListWidget {
    pub fn set_title() {
        // set title, rerender only title part
    }

    pub fn set_list() {
        
    }

    pub fn render() {

    }
}

struct TermConfig {
    out: FakeLock<DynWrite>,
    events: FakeLock<DynEvents>,
    rows: AtomicU16,
    cols: AtomicU16,
}

impl TermConfig {
    pub fn new() -> Self {
        let stdout_raw = io::stdout()
            .into_raw_mode()
            .unwrap();
        let stdout_buf = BufWriter::with_capacity(256, stdout_raw);
        let events = io::stdin()
            .events()
            .map(|res| res.unwrap());
        let size = termion::terminal_size().unwrap();
        Self {
            out: FakeLock::new(Box::new(stdout_buf)),
            events: FakeLock::new(Box::new(events)),
            rows: AtomicU16::new(size.1),
            cols: AtomicU16::new(size.0),
        }
    }

    pub fn restore() {
        let mut out = TERM_CONF.out.lock();
        out.use_color_fg(termion::color::Reset);
        out.use_color_bg(termion::color::Reset);
        out.show_cursor();
        out.flush().unwrap();
    }

    pub fn destroy() {
        let mut out = TERM_CONF.out.lock();
        *out = Box::new(Vec::new());
    }

    pub fn get_out<'a>() -> FakeLockGuard<'a, DynWrite> {
        TERM_CONF.out.lock()
    }

    pub fn get_events<'a>() -> FakeLockGuard<'a, DynEvents> {
        TERM_CONF.events.lock()
    }

    pub fn get_rows() -> u16 {
        TERM_CONF.rows.load(Ordering::Relaxed)
    }

    pub fn get_size() -> (u16, u16) {
        let rows = TERM_CONF.rows.load(Ordering::Relaxed);
        let cols = TERM_CONF.cols.load(Ordering::Relaxed);
        (rows, cols)
    }

    pub fn recalculate_size() -> (u16, u16) {
        let size = termion::terminal_size().unwrap();
        TERM_CONF.rows.store(size.1, Ordering::Relaxed);
        TERM_CONF.cols.store(size.0, Ordering::Relaxed);
        (size.1, size.0)
    }
}

struct AppCache {
    pass: FakeLock<String>,
    index_map: FakeLock<IndexMap>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            pass: FakeLock::new(String::new()),
            index_map: FakeLock::new(HashMap::new())
        }
    }

    pub fn load_globals(pass: &str) -> bool {
        let mut current_pass = APP_CACHE.pass.lock();
        if current_pass.is_empty() || *current_pass != pass {
            if let Ok(map) = secret::read_index_file(pass) {
                let mut index_map = APP_CACHE.index_map.lock();
                *current_pass = pass.to_owned();
                *index_map = map;
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    pub fn get_index_map<'a>() -> FakeLockGuard<'a, IndexMap> {
        APP_CACHE.index_map.lock()
    }
}

struct HistoryStack {
    queue: VecDeque<Page>,
    capacity: usize
}

impl HistoryStack {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { queue: VecDeque::with_capacity(10), capacity }
    }

    pub fn push(&mut self, page: Page) {
        while self.queue.len() > self.capacity - 1 {
            self.queue.pop_back();
        }
        self.queue.push_front(page);
    }

    pub fn pop(&mut self) -> Option<Page> {
        assert!(self.queue.pop_front().unwrap() == Page::Back);
        self.queue.pop_front().unwrap();
        self.queue.pop_front()
    }

    pub fn peek_mut(&mut self) -> &mut Page {
        self.queue.front_mut().unwrap()
    }
}

#[derive(Clone, PartialEq)]
enum Page {
    Password(PasswordPageParams),
    List(ListPageParams),
    Command(CommandPageParams),
    Back,
    Exit,
}

impl Page {
    pub fn render() {
        let init_page = Page::Password(PasswordPageParams);
        let mut history = HistoryStack::with_capacity(3);
        std::iter::repeat(()).try_fold(init_page, |page, _| {
            history.push(page);
            match history.peek_mut() {
                Page::Password(params) => params.render(),
                Page::List(params) => params.render(),
                Page::Command(params) => params.render(),
                Page::Back => history.pop(),
                Page::Exit => None,
            }
        });
    }
}

#[derive(Clone, PartialEq)]
struct PasswordPageParams;

impl PasswordPageParams {
    pub fn render(&self) -> Option<Page> {
        let mut out = TermConfig::get_out();
        let mut pass = String::with_capacity(10);
        out.write_str("password: ");
        out.flush().unwrap();
        TermConfig::get_events().find_map(|event| match event {
            Event::Key(Key::Char('\n')) => {
                if AppCache::load_globals(&pass) {
                    Some(Page::List(ListPageParams::new()))
                } else {
                    out.apply_backspace(pass.len() as u16);
                    out.write_str("incorrect\n");
                    out.move_cursor_to_horiz(0);
                    out.flush().unwrap();
                    Some(Page::Exit)
                }
            }
            Event::Key(Key::Char(char)) => {
                pass.push(char);
                out.write_str("*");
                out.flush().unwrap();
                None
            }
            Event::Key(Key::Backspace) if pass.pop().is_some() => {
                out.apply_backspace(1);
                out.flush().unwrap();
                None
            }
            _ => None
        })
    }
}

#[derive(Clone, PartialEq)]
struct ListPageParams {
    dir: String,
    list: Vec<String>,
    viewport_index: u16,
    selected_index: u16,
}

impl ListPageParams {
    pub fn new() -> Self {
        Self {
            dir: String::new(),
            list: Vec::new(),
            viewport_index: 0,
            selected_index: 0,
        }
    }

    fn load_list(&mut self) {
        let index_map = AppCache::get_index_map();
        self.list = glob::explore_contents(index_map.keys(), &self.dir);
    }

    fn clamp_viewport_index(&mut self) {
        let term_rows = TermConfig::get_rows();
        let viewport_rows = term_rows - 5;
        let total_rows = self.list.len() as u16;
        self.viewport_index = if total_rows > viewport_rows {
            let min = self.selected_index.saturating_sub(viewport_rows - 1);
            let max = total_rows - viewport_rows;
            cmp::max(cmp::min(self.viewport_index, max), min)
        } else {
            0
        };
    }

    fn move_active_selection(&mut self, rows: i8) {
        assert!(rows == 1 || rows == -1, "can only move one up or down");
        let term_rows = TermConfig::get_rows();
        let viewport_rows = term_rows - 5;

        let selected_index = if rows > 0 {
            let new = self.selected_index.saturating_add(1);
            cmp::min(new, self.list.len() as u16 - 1)
        } else {
            self.selected_index.saturating_sub(1)
        };

        let viewport_index = self.viewport_index
            + (selected_index > self.viewport_index + viewport_rows - 1) as u16
            - (selected_index < self.viewport_index) as u16;

        if viewport_index == self.viewport_index {
            let mut out = TermConfig::get_out();
            let prev_row = 2 + self.selected_index - self.viewport_index;
            out.move_cursor_to(prev_row, 2);
            out.write_str(" ");
            let new_row = 2 + selected_index - viewport_index;
            out.move_cursor_to(new_row, 2);
            out.write_str(">");
            out.flush().unwrap();
            self.viewport_index = viewport_index;
            self.selected_index = selected_index;
        } else {
            self.viewport_index = viewport_index;
            self.selected_index = selected_index;
            self.render_explore_contents(false);
            self.render_explore_scroll();
        }
    }

    fn calculate_scroll_state(&self) -> (u16, u16) {
        let term_rows = TermConfig::get_rows();
        let viewport_rows = term_rows - 5;
        let total_rows = self.list.len() as u16;
        let scroll_size = (viewport_rows * viewport_rows) / total_rows;
        let scroll_size = cmp::max(scroll_size, 1);
        let scroll_limit = total_rows - viewport_rows;
        let numerator = self.viewport_index * (viewport_rows - scroll_size);
        let scroll_pos = (2 * numerator + scroll_limit) / (2 * scroll_limit);
        (scroll_size, scroll_pos)
    }

    fn render_before_all(&self) {
        let mut out = TermConfig::get_out();
        out.clear_screen();
        out.hide_cursor();
    }

    fn render_explore_contents(&self, clear_line: bool) {
        let mut out = TermConfig::get_out();
        let (term_rows, term_cols) = TermConfig::get_size();
        let viewport_rows = term_rows - 5;
        let viewport_cols = term_cols / 2 - 6;
        out.move_cursor_to(2, 0);
        (0..viewport_rows).for_each(|i| {
            if self.viewport_index + i == self.selected_index {
                out.move_cursor_to_horiz(2);
                out.write_str("> ");
            } else {
                out.move_cursor_to_horiz(4);
            }
            let list_index = (self.viewport_index + i) as usize;
            if let Some(key) = self.list.get(list_index) {
                out.write_str(key);
                let prev_index = list_index.saturating_sub(1);
                let next_index = cmp::min(list_index + 1, self.list.len() - 1);
                let max_len = self.list[prev_index..=next_index].iter()
                    .fold(0, |res, item| cmp::max(res, item.len()));
                if clear_line {
                    let remaining = viewport_cols as usize - key.len();
                    out.write_str(" ".repeat(remaining));
                } else {
                    out.write_str(" ".repeat(max_len - key.len()));
                }
            } else if clear_line {
                out.write_str(" ".repeat(viewport_cols as usize));
            }
            out.move_cursor_down(1);
        });
        out.flush().unwrap();
    }

    fn render_explore_scroll(&self) {
        let (term_rows, term_cols) = TermConfig::get_size();
        let viewport_rows = term_rows - 5;
        let total_rows = self.list.len() as u16;
        let (scroll_size, scroll_pos) = match total_rows > viewport_rows {
            true => self.calculate_scroll_state(),
            false => (0, 0),
        };
        let mut out = TermConfig::get_out();
        out.move_cursor_to(2, term_cols / 2 - 3);
        (0..viewport_rows).for_each(|i| {
            let char = match i >= scroll_pos && i < scroll_pos + scroll_size {
                true => "\u{2502}",
                false => " ",
            };
            out.write_str(char);
            out.move_cursor_down(1);
            out.move_cursor_left(1);
        });
        out.flush().unwrap();
    }

    fn render_explore_window(&self) {
        let mut out = TermConfig::get_out();
        let (term_rows, term_cols) = TermConfig::get_size();
        out.move_cursor_to(0, 0);
        out.draw_box(term_rows - 1, term_cols / 2);
        out.draw_window_title(&format!("explore /{}", self.dir));
        out.flush().unwrap();
    }

    fn render_preview_window(&self) {
        let mut out = TermConfig::get_out();
        let (term_rows, term_cols) = TermConfig::get_size();
        let window_cols = term_cols / 2 + term_cols % 2 - 1;
        out.move_cursor_to(0, term_cols / 2);
        out.draw_box(term_rows - 1, window_cols);
        out.draw_window_title(&format!("preview /{}", self.dir));
        out.flush().unwrap();
    }

    pub fn render(&mut self) -> Option<Page> {
        self.load_list();
        self.clamp_viewport_index();
        self.render_before_all();
        self.render_explore_window();
        self.render_explore_contents(false);
        self.render_explore_scroll();
        self.render_preview_window();
        TermConfig::get_events().find_map(|event| match event {
            Event::Key(Key::Up) => {
                self.move_active_selection(-1);
                None
            }
            Event::Key(Key::Down) => {
                self.move_active_selection(1);
                None
            }
            Event::Key(Key::Char('\n')) => {
                // let pass = pass.to_owned();
                // Some(Page::List { pass, dir: dir.to_owned() + "a" })
                None
            }
            Event::Key(Key::Char(':')) => {
                Some(Page::Command(CommandPageParams))
            }
            Event::Key(Key::Char(char)) => {
                self.dir.push(char);
                self.load_list();
                self.viewport_index = 0;
                self.selected_index = 0;
                self.render_explore_window();
                self.render_explore_contents(true);
                self.render_explore_scroll();
                None
            }
            _ => None
        })
    }
}

#[derive(Clone, PartialEq)]
struct CommandPageParams;

impl CommandPageParams {
    fn evaluate_command(command: &str) -> Option<Page> {
        match command {
            "q" => {
                let mut out = TermConfig::get_out();
                out.move_cursor_to(0, 0);
                out.clear_screen();
                out.flush().unwrap();
                Some(Page::Exit)
            }
            "r" => {
                let (rows, cols) = TermConfig::recalculate_size();
                let message = format!("resized to {} x {}", cols, rows);
                Self::render_result(&message, true);
                None
            }
            cmd => {
                let message = format!("unrecognized command: {}", cmd);
                Self::render_result(&message, false);
                None
            }
        }
    }

    fn render_result(message: &str, success: bool) {
        let mut out = TermConfig::get_out();
        match success {
            true => out.use_color_fg(termion::color::Green),
            false => out.use_color_fg(termion::color::Red),
        }
        out.hide_cursor();
        out.move_cursor_to_horiz(0);
        out.write_str(" ");
        out.write_str(message);
        out.flush().unwrap();
    }

    fn render_before_all() {
        let mut out = TermConfig::get_out();
        let (term_rows, term_cols) = TermConfig::get_size();
        out.move_cursor_to(term_rows - 1, 0);
        out.write_str(" ".repeat(term_cols as usize));
        out.move_cursor_to_horiz(0);
        out.use_color_fg(termion::color::Yellow);
        out.write_str(":");
        out.show_cursor();
        out.flush().unwrap();
    }

    fn render_after_all() {
        let mut out = TermConfig::get_out();
        out.use_color_fg(termion::color::Reset);
        out.flush().unwrap();
    }

    pub fn render(&self) -> Option<Page> {
        let mut command = String::with_capacity(20);
        let mut editing = true;
        Self::render_before_all();
        TermConfig::get_events().find_map(|event| match event {
            Event::Key(Key::Esc) => {
                Self::render_after_all();
                Some(Page::Back)
            },
            Event::Key(Key::Char('\n')) if editing => {
                editing = false;
                Self::evaluate_command(&command)
            }
            Event::Key(Key::Char(char)) if editing => {
                if command.len() < 20 {
                    let mut out = TermConfig::get_out();
                    out.write_all(&[char as u8]).unwrap();
                    out.flush().unwrap();
                    command.push(char);
                }
                None
            }
            Event::Key(Key::Backspace) if editing => {
                if command.pop().is_some() {
                    let mut out = TermConfig::get_out();
                    out.apply_backspace(1);
                    out.flush().unwrap();
                }
                None
            }
            _ => None
        })
    }
}

pub fn start_app() {
    std::panic::catch_unwind(Page::render)
        .unwrap_or_else(|_| TermConfig::restore());
    TermConfig::destroy();
}

// TODO: remove this snippet
/*
let mut log = std::fs::OpenOptions::new()
    .write(true)
    .append(true)
    .create(true)
    .open("debug.log")
    .unwrap();
*/
