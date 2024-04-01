use std::cmp;
use termion::color;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use crate::highlighting;

#[derive(Default)]
#[derive(Clone)]
pub struct Row {
    string: String,
    len: usize,
    highlighting: Vec<highlighting::Type>
}

impl From<&str> for Row {
    fn from(slice: &str) -> Self {
        let mut row = Self {
            string: String::from(slice),
            len: 0,
            highlighting: Vec::new()
        };
        row.update_len();
        row
    }
}

impl Row {
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.string.len());
        let start = cmp::min(start, end);
        let mut current_highlight = &highlighting::Type::None;
        let mut result = String::new();
        #[allow(clippy::arithmetic_side_effects)]
        for (index, grapheme) in self.string[..].graphemes(true).enumerate().skip(start).take(end-start) {
            if let Some(c) = grapheme.chars().next() {
                let highlighting_type = self.highlighting.get(index).unwrap_or(&highlighting::Type::None);
                if highlighting_type != current_highlight {
                    current_highlight = highlighting_type;
                    let start_highlighting = format!("{}", termion::color::Fg(highlighting_type.to_color()));
                    result.push_str(&start_highlighting[..]);
                }
                if grapheme == "\t" {
                    result.push_str(" ");
                } else {
                    result.push(c);
                }
            }
        }
        let end_highlighting = format!("{}", termion::color::Fg(color::Reset));
        result.push_str(&end_highlighting[..]);
        result
    }
    pub fn insert(&mut self, x_position: usize, c: char) {
        if x_position >= self.len() {
            self.string.push(c);
        } else {
            let mut result: String = self.string[..].graphemes(true).take(x_position).collect();
            let split: String = self.string[..].graphemes(true).skip(x_position).collect();
            result.push(c);
            result.push_str(&split);
            self.string = result;
        }
        self.update_len();
    }
    pub fn append(&mut self, new: &Self) {
        self.string = format!("{}{}", self.string, new.string);
        self.update_len()
    }
    pub fn find(&self, query: &str) -> Option<usize> {
        let matching_byte_index = self.string.find(query);
        if let Some(matching_byte_index) = matching_byte_index {
            for (grapheme_index, (byte_index, _)) in self.string[..].grapheme_indices(true).enumerate() {
                if matching_byte_index == byte_index {
                    return Some(grapheme_index);
                }
            }
        }
        None
    }
    #[allow(clippy::arithmetic_side_effects)]
    pub fn delete(&mut self, at: usize) {
        if at >= self.len() {
            return 
        }
        let mut result: String = self.string[..].graphemes(true).take(at).collect();
        let split: String = self.string[..].graphemes(true).skip(at.saturating_add(1)).collect();
        result.push_str(&split);
        self.string = result;
        
        self.update_len();
    }
    pub fn split(&mut self, at: usize) -> Self {
        let result: String = self.string[..].graphemes(true).take(at).collect();
        let new_row: String = self.string[..].graphemes(true).skip(at).collect();
        self.string = result;
        self.update_len();
        Self::from(&new_row[..])
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn update_len(&mut self) {
        self.len = self.string[..].graphemes(true).count();
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }
    pub fn is_equal(&self, line: &str) -> bool {
        self.string == line
    }
    pub fn highlight(&mut self, query: Option<&String>) {
        let mut highlighting = Vec::new();
        let chars: Vec<char> = self.string.chars().collect();
        let mut matches = Vec::new();
        let mut search_index = 0;

        if let Some(query) = query {
            if let Some(search_match) = self.find(query) {
                matches.push(search_match);
            }
        }

        let mut index = 0;
        while let Some(c) = chars.get(index) {
            if let Some(query) = query {
                if matches.contains(&index) {
                    for _ in query[..].graphemes(true) {
                        index += 1;
                        highlighting.push(highlighting::Type::Match);
                    }
                    continue;
                }
            }

            if c.is_ascii_digit() {
                highlighting.push(highlighting::Type::Number);
            } else {
                highlighting.push(highlighting::Type::None);
            }
            index += 1;
        }

        self.highlighting = highlighting;
    }
}