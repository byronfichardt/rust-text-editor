use crate::Document;
use crate::Row;
use crate::Terminal;
use std::env;
use std::time::Duration;
use std::time::Instant;
use termion::color;
use termion::event::Key;
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::util::as_24_bit_terminal_escaped;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);

// this is pretty cool i think something
enum EditorMode {
    Normal,
    CtrlXPressed,
}
pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage,
    mode: EditorMode,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            text: message,
            time: Instant::now(),
        }
    }
}

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Editor {
    pub fn run(&mut self) {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        loop {
            // this is so the screen is refreshed every time the loop runs
            if let Err(error) = self.refresh_screen(&ps, &ts) {
                die(&error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status =
            String::from("HELP: Ctrl-F = find | Ctrl-S = save | Ctrl-C = quit");
        let document = if args.len() > 1 {
            let file_name = &args[1];
            let doc = Document::open(file_name);
            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("ERR: Could not open file: {file_name}");
                Document::default()
            }
        } else {
            Document::default()
        };
        Self {
            should_quit: false,
            terminal: Terminal::default().expect("failed to initialize terminal"),
            cursor_position: Position::default(),
            offset: Position::default(),
            document,
            status_message: StatusMessage::from(initial_status),
            mode: EditorMode::Normal,
        }
    }
    fn refresh_screen(&mut self, ps: &SyntaxSet, ts: &ThemeSet) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::clear_screen();
            println!("Goodbye.\r");
        } else {
            self.draw_rows(ps, ts);
            self.draw_status_bar();
            self.draw_message_bar();
            let x = self.cursor_position.x.saturating_sub(self.offset.x);
            let y = self.cursor_position.y.saturating_sub(self.offset.y);
            Terminal::cursor_position(&Position { x, y });
        }
        Terminal::cursor_show();
        Terminal::flush()
    }
    fn draw_status_bar(&mut self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let modified_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };
        let mut file_name = "[No_Name]".to_string();
        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }
        status = format!(
            "{} - {} lines{}",
            file_name,
            self.document.len(),
            modified_indicator
        );
        let line_number = self.cursor_position.y.saturating_add(1);
        let document_length = self.document.len();
        let line_indicator = format!("{line_number}/{document_length}");
        #[allow(clippy::arithmetic_side_effects)]
        let len = status.len() + line_indicator.len();
        status.push_str(&" ".repeat(width.saturating_sub(len)));
        status = format!("{status}{line_indicator}");
        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{status}\r");
        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }
    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        if Instant::now() - message.time < Duration::new(5, 0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{text}");
        }
    }
    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Esc => match self.mode {
                EditorMode::CtrlXPressed => self.mode = EditorMode::Normal,
                _ => (),
            },
            Key::Ctrl('c') => {
                if self.document.is_dirty() {
                    self.dirty_quit()?;
                } else {
                    self.should_quit = true;
                }
            }
            Key::Ctrl('x') => {
                self.mode = EditorMode::CtrlXPressed;
            }
            Key::Ctrl('d') => {
                // remove line at cursor
                self.document.delete_row(self.cursor_position.y)
            }
            Key::Ctrl('f') => self.search(),
            Key::Ctrl('s') => self.save(),
            Key::Char(c) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(Key::Right);
            }
            Key::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_position);
                }
            }
            Key::Delete => {
                self.document.delete(&self.cursor_position);
            }
            Key::Up | Key::Down => match self.mode {
                EditorMode::CtrlXPressed => self.move_row(pressed_key),
                EditorMode::Normal => self.move_cursor(pressed_key),
            },
            Key::Left | Key::Right | Key::PageDown | Key::PageUp | Key::End | Key::Home => {
                self.move_cursor(pressed_key)
            }
            _ => (),
        }
        self.scroll();
        Ok(())
    }

    fn search(&mut self) {
        if let Some(query) = self.prompt("Search: ").unwrap_or(None) {
            if let Some(position) = self.document.find(&query[..], &self.cursor_position) {
                self.cursor_position = position;
            } else {
                self.status_message = StatusMessage::from(format!("Not found :{}.", query));
            }
        }
    }
    fn move_row(&mut self, key: Key) {
        let Position { x: _, y } = self.cursor_position;
        if let Some(row) = self.document.row(y) {
            let new_row = row.clone();
            self.document.delete_row(y);
            match key {
                Key::Up => {
                    if y > 0 {
                        self.document.insert_row(new_row, y - 1);
                    }
                }
                Key::Down => {
                    if y + 1 <= self.document.len() {
                        self.document.insert_row(new_row, y + 1);
                    }
                }
                _ => (),
            }
            self.move_cursor(key)
        }
    }
    fn dirty_quit(&mut self) -> Result<(), std::io::Error> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        loop {
            self.status_message = StatusMessage::from(
                "You will loose unsaved changes, enter to quit? esc to continue.".to_string(),
            );
            self.refresh_screen(&ps, &ts)?;
            match Terminal::read_key()? {
                Key::Char(c) => {
                    if c == '\n' {
                        self.should_quit = true;
                        break;
                    }
                }
                Key::Esc => {
                    self.status_message = StatusMessage::from(String::from(
                        "HELP: Ctrl-F = find | Ctrl-S = save | Ctrl-C = quit",
                    ));
                    break;
                }
                _ => (),
            }
        }
        Ok(())
    }
    fn save(&mut self) {
        if self.document.file_name.is_none() {
            let new_name = self.prompt("Save As: ").unwrap_or(None);
            if new_name.is_none() {
                self.status_message = StatusMessage::from("Save aborted.".to_string());
                return;
            }
            self.document.file_name = new_name;
        }
        if self.document.save().is_ok() {
            self.status_message = StatusMessage::from("File saved successfully.".to_string());
        } else {
            self.status_message = StatusMessage::from("Error writing file!".to_string());
        }
    }
    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let height = self.terminal.size().height as usize;
        let width = self.terminal.size().width as usize;
        let offset = &mut self.offset;
        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }
    fn move_cursor(&mut self, key: Key) {
        let Position { mut x, mut y } = self.cursor_position;
        let height = self.document.len();
        let terminal_height = self.terminal.size().height as usize;
        let width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            Key::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            Key::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            }
            Key::PageUp => {
                y = if y > terminal_height {
                    y.saturating_sub(terminal_height)
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y.saturating_add(terminal_height)
                } else {
                    height
                }
            }
            Key::End => x = width,
            Key::Home => x = 0,
            _ => (),
        }
        let end_of_row = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        if x > end_of_row {
            x = end_of_row;
        }
        self.cursor_position = Position { x, y }
    }
    fn draw_row(&self, row: &Row, ps: &SyntaxSet, ts: &ThemeSet) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x.saturating_add(width);
        let row = row.render(start, end);

        let syntax = ps.find_syntax_by_extension("rs").unwrap();
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        let ranges: Vec<(Style, &str)> = h.highlight_line(row.as_str(), &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        println!("{escaped}\r");
    }
    fn draw_rows(&self, ps: &SyntaxSet, ts: &ThemeSet) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.y.saturating_add(terminal_row as usize))
            {
                self.draw_row(row, ps, ts);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                println!("Byron's Code Editor -- version {VERSION}\r");
            } else {
                println!("~\r");
            }
        }
    }
    fn prompt(&mut self, prompt: &str) -> Result<Option<String>, std::io::Error> {
        let mut result = String::new();
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        loop {
            self.status_message = StatusMessage::from(format!("{prompt}{result}"));
            self.refresh_screen(&ps, &ts)?;
            match Terminal::read_key()? {
                Key::Backspace => result.truncate(result.len().saturating_sub(1)),
                Key::Ctrl('c') | Key::Esc => {
                    result.truncate(0);
                    break;
                }
                Key::Char(c) => {
                    if c == '\n' {
                        break;
                    }
                    result.push(c);
                }
                _ => (),
            }
        }
        self.status_message = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(result))
    }
}

fn die(e: &std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}
