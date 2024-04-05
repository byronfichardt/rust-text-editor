use std::{
    fs,
    io::{Error, Write},
};

use crate::{Position, Row};
use syntect::{easy::HighlightLines, parsing::SyntaxSet};
use syntect::{
    highlighting::{Style, ThemeSet},
    util::as_24_bit_terminal_escaped,
};

#[derive(Default)]
pub struct Document {
    rows: Vec<Row>,
    pub file_name: Option<String>,
    dirty: bool,
}

impl Document {
    pub fn open(filename: &str) -> Result<Self, std::io::Error> {
        let file = fs::read_to_string(filename)?;
        let mut rows = Vec::new();
        let ps = SyntaxSet::load_defaults_nonewlines();
        let ts = ThemeSet::load_defaults();
        let syntax = ps.find_syntax_by_extension("rs").unwrap();
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        for line in file.lines() {
            let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
            let line = Row::from(escaped.as_str());
            rows.push(line)
        }

        Ok(Self {
            rows,
            file_name: Some(filename.to_string()),
            dirty: false,
        })
    }
    pub fn is_dirty(&mut self) -> bool {
        self.dirty
    }
    pub fn insert(&mut self, at: &Position, c: char) {
        if at.y > self.len() {
            return;
        }
        self.dirty = true;
        if c == '\n' {
            self.insert_newline(at);
            return;
        }
        // if the position y is equal to the length of the document we add a new row
        if at.y == self.len() {
            let mut row = Row::default();
            row.insert(0, c);
            self.rows.push(row);
        } else if at.y < self.len() {
            let row = self.rows.get_mut(at.y).unwrap();
            row.insert(at.x, c);
        }
    }
    pub fn find(&mut self, query: &str, cursor_position: &Position) -> Option<Position> {
        for (y, row) in self.rows.iter().enumerate().skip(cursor_position.y) {
            if let Some(x) = row.find(query) {
                return Some(Position { x, y });
            }
        }
        None
    }
    fn insert_newline(&mut self, at: &Position) {
        if at.y > self.len() {
            return;
        }
        if at.y == self.len() {
            self.rows.push(Row::default());
            return;
        }
        let current_row = &mut self.rows[at.y];
        let mut new_row = current_row.split(at.x);
        #[allow(clippy::arithmetic_side_effects)]
        self.rows.insert(at.y + 1, new_row)
    }
    #[allow(clippy::arithmetic_side_effects)]
    pub fn delete(&mut self, at: &Position) {
        self.dirty = true;
        let len = self.len();
        if at.y >= len {
            return;
        }
        if at.x == self.rows.get_mut(at.y).unwrap().len() && at.y + 1 < len {
            let next_row = self.rows.remove(at.y + 1);
            let row = self.rows.get_mut(at.y).unwrap();
            row.append(&next_row);
        } else {
            let row = self.rows.get_mut(at.y).unwrap();
            row.delete(at.x);
        }
    }
    pub fn delete_row(&mut self, at: usize) {
        self.dirty = true;
        self.rows.remove(at);
    }
    pub fn insert_row(&mut self, mut row: Row, at: usize) {
        self.dirty = true;
        self.rows.insert(at, row)
    }
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(file_name) = &self.file_name {
            let mut file = fs::File::create(file_name)?;
            for row in &self.rows {
                file.write_all(row.as_bytes())?;
                file.write(b"\n")?;
            }
            self.dirty = false;
        }
        Ok(())
    }
}
