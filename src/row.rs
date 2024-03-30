use std::cmp;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
#[derive(Clone)]
pub struct Row {
    string: String,
    len: usize
}

impl From<&str> for Row {
    fn from(slice: &str) -> Self {
        let mut row = Self {
            string: String::from(slice),
            len: 0,
        };
        row.update_len();
        row
    }
}

impl Row {
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.string.len());
        let start = cmp::min(start, end);
        let mut result = String::new();
        #[allow(clippy::arithmetic_side_effects)]
        for grapheme in self.string[..].graphemes(true).skip(start).take(end-start) {
            if grapheme == "\t" {
                result.push_str(" ");
            } else {
                result.push_str(grapheme);
            }
        }
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
}