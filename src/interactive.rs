use crate::secret;
use crate::util::{WriteExt, WriteUiExt};
use std::io::{self, Write, BufWriter};
use termion::event::{Key, Event};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

#[derive(Clone)]
enum Page {
    PasswordPrompt,
    Command(CommandPageParams),
    List(ListPageParams),
    Exit,
}

#[derive(Clone)]
struct CommandPageParams {
    back: Box<Page>,
}

#[derive(Clone)]
struct ListPageParams {
    pass: String,
    dir: String,
}

fn get_output_stream() -> impl Write {
    let stdout_raw = io::stdout()
        .into_raw_mode()
        .unwrap();
    BufWriter::new(stdout_raw)
}

fn get_event_stream() -> impl Iterator<Item = Event> {
    io::stdin()
        .events()
        .map(|res| res.unwrap())
}

fn get_terminal_size() -> (u16, u16) {
    termion::terminal_size().unwrap()
}

fn handle_page_password() -> Option<Page> {
    let mut out = get_output_stream();
    let mut pass = String::with_capacity(10);
    out.write_str("password: ");
    out.flush().unwrap();
    get_event_stream().find_map(|event| match event {
        Event::Key(Key::Char('\n')) => {
            if secret::read_index_file(&pass).is_ok() {
                let params = ListPageParams {
                    pass: pass.to_owned(),
                    dir: String::new()
                };
                Some(Page::List(params))
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

fn handle_page_command(params: CommandPageParams) -> Option<Page> {
    let mut out = get_output_stream();
    let mut command = String::with_capacity(20);
    let mut editing = true;
    let (term_w, term_h) = get_terminal_size();
    out.move_cursor_to(term_h - 1, 0);
    out.use_color_fg(termion::color::Yellow);
    out.write_str(" ".repeat(term_w as usize));
    out.move_cursor_to_horiz(0);
    out.write_str(":");
    out.flush().unwrap();
    get_event_stream().find_map(move |event| match event {
        Event::Key(Key::Esc) => {
            out.use_color_fg(termion::color::Reset);
            Some(*params.back.clone())
        }
        Event::Key(Key::Char('\n')) if editing => match command.as_ref() {
            "q" => {
                out.use_color_fg(termion::color::Reset);
                out.use_color_bg(termion::color::Reset);
                out.move_cursor_to(0, 0);
                out.clear_screen();
                out.show_cursor();
                out.flush().unwrap();   
                Some(Page::Exit)
            },
            _ => {
                editing = false;
                None
            }
            // show the result (success / error) in this line
        }
        Event::Key(Key::Char(char)) if editing => {
            command.push(char);
            out.write_all(&[char as u8]).unwrap();
            out.flush().unwrap();
            None
        }
        Event::Key(Key::Backspace) if editing && command.pop().is_some() => {
            out.apply_backspace(1);
            out.flush().unwrap();
            None
        }
        _ => None
    })
}

fn handle_page_list(params: ListPageParams) -> Option<Page> {
    let mut out = get_output_stream();
    let _index_map = secret::read_index_file(&params.pass).unwrap();
    let (term_w, term_h) = get_terminal_size();
    out.hide_cursor();
    out.clear_screen();
    out.move_cursor_to(0, 0);
    out.draw_box(term_w / 2, term_h - 1);
    out.draw_window_title(&format!("explore /{}", params.dir));
    out.move_cursor_to(0, term_w / 2);
    out.draw_box(term_w / 2 + term_w % 2 - 1, term_h - 1);
    out.draw_window_title(&format!("preview /{}", params.dir));
    out.flush().unwrap();
    get_event_stream().find_map(|event| match event {
        Event::Key(Key::Up) => {
            None
        }
        Event::Key(Key::Down) => {
            None
        }
        Event::Key(Key::Char('\n')) => {
            // let pass = pass.to_owned();
            // Some(Page::List { pass, dir: dir.to_owned() + "a" })
            None
        }
        Event::Key(Key::Char(':')) => {
            let back = Box::new(Page::List(params.clone()));
            let command_params = CommandPageParams { back };
            Some(Page::Command(command_params))
        }
        _ => None
    })
}

// fn handle_page_exit(params: bool) -> Option<Page> {
//     
// }

pub fn start_app() {
    std::iter::repeat(())
        .try_fold(Page::PasswordPrompt, |page, _| match page {
            Page::PasswordPrompt => handle_page_password(),
            Page::Command(params) => handle_page_command(params),
            Page::List(params) => handle_page_list(params),
            Page::Exit => None,
        });
}
