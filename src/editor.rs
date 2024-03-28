use crate::Terminal;
use crate::Document;
use crate::Row;
use std::env;
use std::io::Read;
use std::io::Stdin;
use termion::color;
use std::time::Duration;
use std::time::Instant;
use termion::event::Key;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);
// this is pretty cool i think
pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage
}

struct StatusMessage {
    text: String,
    time: Instant
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            text: message,
            time: Instant::now()
        }
    }
}

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize
}

impl Editor {
    pub fn run(&mut self) {
        loop {
            // this is so the screen is refreshed every time the loop runs
            if let Err(error) = self.refresh_screen() {
                die(error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(error);
            }
        }
    }
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from("HELP: Ctrl-S = save | Ctrl-C = quit");
        let document = if args.len() > 1 {
            let file_name = &args[1];
            let doc = Document::open(&file_name);
            if doc.is_ok() {
                doc.unwrap()
            } else {
                initial_status = format!("ERR: Could not open file: {}", file_name);
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
            document: document,
            status_message: StatusMessage::from(initial_status)
        }
    }
    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::clear_screen();
            println!("Goodbye.\r");
        } else {
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            let x = self.cursor_position.x.saturating_sub(self.offset.x);
            let y = self.cursor_position.y.saturating_sub(self.offset.y);
            Terminal::cursor_position(&Position {
                x,
                y
            });
        }
        Terminal::cursor_show();
        Terminal::flush()
    }
    fn draw_status_bar(&self) {
        let spaces = " ".repeat(self.terminal.size().width as usize);
        let mut status;
        let width = self.terminal.size().width as usize;
        let mut file_name = "[No_Name]".to_string();
        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }
        status = format!("{} - {} lines", file_name, self.document.len());
        let line_number = self.cursor_position.y.saturating_add(1);
        let document_length = self.document.len();
        let line_indicator = format!("{}/{}", line_number, document_length);
        let len = status.len() + line_indicator.len();
        if width > len {
            status.push_str(&" ".repeat(width - len))
        }
        status = format!("{}{}", status, line_indicator);
        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{}\r", status);
        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }
    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        if Instant::now() - message.time < Duration::new(5,0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text)
        }
    }
    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let Position {mut x, mut y} = self.cursor_position;
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Ctrl('c') => self.should_quit = true,
            Key::Ctrl('s') => {
                // if self.document.file_name.is_none() {
                //     self.document.file_name = Some(self.prompt("Save As: ")?);
                // }
                if self.document.save().is_ok() {
                    self.status_message = StatusMessage::from("File saved successfully.".to_string());
                } else {
                    self.status_message = StatusMessage::from("Error writing file!".to_string());
                }
            }
            Key::Char(c) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(Key::Right)
            },
            Key::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_position);
                }
            }
            Key::Delete => {
                self.document.delete(&self.cursor_position);
            }
            Key::Up 
            | Key::Down 
            | Key::Left 
            | Key::Right 
            | Key::PageDown 
            | Key::PageUp 
            | Key::End
            | Key::Home => self.move_cursor(pressed_key),
            _ => (),
        }
        self.scroll();
        Ok(())
    }
    fn scroll(&mut self) {
        let Position {x, y} = self.cursor_position;
        let height = self.terminal.size().height as usize;
        let width = self.terminal.size().width as usize;
        let mut offset = &mut self.offset;
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
        let Position {mut x, mut y} = self.cursor_position;
        let height = self.document.len();
        let terminal_height = self.terminal.size().height as usize;
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1)
                } 
            },
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
            },
            Key::Right => {
                if x < width {
                    x += 1 
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            },
            Key::PageUp => {
                y = if y > terminal_height {
                    y - terminal_height
                } else {
                    0
                }
            },
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y + terminal_height as usize
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
        self.cursor_position = Position {x, y}
    }
    fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x + width;
        let row = row.render(start, end);
        println!("{}\r", row)
    }
    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self.document.row(terminal_row as usize + self.offset.y) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                println!("Byron's Code Editor -- version {}\r", VERSION);
            }else {
                println!("~\r");
            }
        }
    }
    // fn prompt(&mut self, prompt: &str) -> Result<String, std::io::Error> {
    //     let mut input = String::new();
    //     let prompt_input = std::io::stdin().read_line(&mut input)?;
    //     let status = StatusMessage::from(format!("{prompt}{prompt_input}"));

    //     self.status_message = status;
    //     Ok(())

    // }
}

fn die(e: std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}