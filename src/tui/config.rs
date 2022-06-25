use crate::util::sync::{SingleLock, SingleLockGuard};
use once_cell::sync::Lazy;
use std::io::{self, Write, BufWriter};
use std::sync::atomic::{AtomicU16, Ordering};
use super::ansi::{Color, TermControl};
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

type DynWrite = Box<dyn Write + Send + Sync>;
type DynEvents = Box<dyn Iterator<Item = Event> + Send + Sync>;

static TERM_CONF: Lazy<TermConfig> = Lazy::new(TermConfig::new);

pub struct TermConfig {
    out: SingleLock<DynWrite>,
    events: SingleLock<DynEvents>,
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
            out: SingleLock::new(Box::new(stdout_buf)),
            events: SingleLock::new(Box::new(events)),
            rows: AtomicU16::new(size.1),
            cols: AtomicU16::new(size.0),
        }
    }

    pub fn restore() {
        let mut out = TERM_CONF.out.lock();
        out.use_color_fg(Color::Reset);
        out.use_color_bg(Color::Reset);
        out.show_cursor();
        out.flush().unwrap();
    }

    pub fn destroy() {
        let mut out = TERM_CONF.out.lock();
        *out = Box::new(Vec::new());
    }

    pub fn get_out<'a>() -> SingleLockGuard<'a, DynWrite> {
        TERM_CONF.out.lock()
    }

    pub fn get_events<'a>() -> SingleLockGuard<'a, DynEvents> {
        TERM_CONF.events.lock()
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
