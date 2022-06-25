use crate::secret::{self, IndexMap};
use crate::util::pattern::PatternFilter;
use crate::util::sync::{SingleLock, SingleLockGuard};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use super::config::TermConfig;
use super::ansi::{Color, TermControl};
use super::widget::ListWidgetBuilder;
use termion::event::{Key, Event};
// use termion::input::TermRead;
// use termion::raw::IntoRawMode;

static GLOBAL_CACHE: Lazy<GlobalCache> = Lazy::new(GlobalCache::new);

struct GlobalCache {
    pass: SingleLock<String>,
    index_map: SingleLock<IndexMap>,
}

impl GlobalCache {
    pub fn new() -> Self {
        Self {
            pass: SingleLock::new(String::new()),
            index_map: SingleLock::new(IndexMap::new())
        }
    }

    pub fn load_protected(pass: &str) -> bool {
        let mut current_pass = GLOBAL_CACHE.pass.lock();
        if current_pass.is_empty() || *current_pass != pass {
            if let Ok(map) = secret::read_index_file(pass) {
                let mut index_map = GLOBAL_CACHE.index_map.lock();
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

    pub fn get_index_map<'a>() -> SingleLockGuard<'a, IndexMap> {
        GLOBAL_CACHE.index_map.lock()
    }
}

struct HistoryStack {
    queue: VecDeque<Page>,
    capacity: usize
}

impl HistoryStack {
    pub fn with_max_capacity(capacity: usize) -> Self {
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

#[derive(PartialEq)]
enum Page {
    Password(PasswordPageParams),
    List(ListPageParams),
    Command(CommandPageParams),
    Back,
    Exit,
}

impl Page {
    pub fn render(self) {
        let mut history = HistoryStack::with_max_capacity(5);
        std::iter::repeat(()).try_fold(self, |page, _| {
            history.push(page);
            match history.peek_mut() {
                Self::Password(params) => params.render(),
                Self::List(params) => params.render(),
                Self::Command(params) => params.render(),
                Self::Back => history.pop(),
                Self::Exit => None,
            }
        });
    }
}

#[derive(PartialEq)]
struct PasswordPageParams;

impl PasswordPageParams {
    pub fn render(&self) -> Option<Page> {
        let mut out = TermConfig::get_out();
        let mut pass = String::with_capacity(10);
        out.write_str("password: ");
        out.flush().unwrap();
        TermConfig::get_events().find_map(|event| match event {
            Event::Key(Key::Char('\n')) => {
                if GlobalCache::load_protected(&pass) {
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

#[derive(PartialEq)]
struct ListPageParams {
    dir: String,
    selected_index: u16,
    viewport_index: u16,
}

impl ListPageParams {
    pub fn new() -> Self {
        Self {
            dir: String::new(),
            selected_index: 0,
            viewport_index: 0,
        }
    }

    fn render_before_all(&self) {
        let mut out = TermConfig::get_out();
        out.clear_screen();
        out.hide_cursor();
    }

    pub fn render(&mut self) -> Option<Page> {
        let (term_rows, term_cols) = TermConfig::get_size();
        let index_map = GlobalCache::get_index_map();
        let mut explore_list = ListWidgetBuilder {
            start: (0, 0),
            size: (term_rows - 1, term_cols / 2),
            title: format!("explore /{}", self.dir),
            list: index_map.keys().explore_contents(&self.dir),
            selected_index: self.selected_index,
            viewport_index: self.viewport_index,
        }.build();
        // let mut secret_view = 
        self.render_before_all();
        explore_list.render();
        TermConfig::get_events().find_map(|event| match event {
            Event::Key(Key::Up) => {
                explore_list.select_prev();
                None
            }
            Event::Key(Key::Down) => {
                explore_list.select_next();
                None
            }
            Event::Key(Key::Backspace) => {
                let popped = self.dir.pop();
                if popped == Some('/') {
                    self.dir.push('/')
                } else if popped.is_some() {
                    let list = index_map.keys().explore_contents(&self.dir);
                    explore_list.set_list(list);
                    explore_list.set_title(format!("explore /{}", self.dir));
                }
                None
            }
            Event::Key(Key::Char('\n')) => {
                // let pass = pass.to_owned();
                // Some(Page::List { pass, dir: dir.to_owned() + "a" })
                None
            }
            Event::Key(Key::Char(':')) => {
                self.selected_index = explore_list.get_selected_index();
                self.viewport_index = explore_list.get_viewport_index();
                Some(Page::Command(CommandPageParams))
            }
            Event::Key(Key::Char(char)) if char != '/' => {
                self.dir.push(char);
                let list = index_map.keys().explore_contents(&self.dir);
                explore_list.set_list(list);
                explore_list.set_title(format!("explore /{}", self.dir));
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
        out.use_color_fg(if success { Color::Green } else { Color::Red });
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
        out.use_color_fg(Color::Yellow);
        out.write_str(":");
        out.show_cursor();
        out.flush().unwrap();
    }

    fn render_after_all() {
        let mut out = TermConfig::get_out();
        out.use_color_fg(Color::Reset);
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

pub fn start_event_loop_blocking() {
    let init_page = Page::Password(PasswordPageParams);
    std::panic::catch_unwind(|| init_page.render())
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
