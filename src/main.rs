#![warn(clippy::all, clippy::pedantic)]
mod editor;
mod terminal;
mod document;
mod row;

use editor::Editor;
pub use document::Document;
pub use row::Row;
pub use terminal::Terminal;
pub use editor::Position;

fn main() {
    Editor::default().run();
}

