//! Where text-level conventions apply.
//!
//! Working notes and ITA-MD directives are line- and text-level conventions,
//! but they must never fire inside code (fenced or indented blocks, code
//! spans) or inside HTML comments: a handbook that shows a directive as an
//! example, or a commented-out draft, is not a directive. The document's own
//! Markdown parse decides what is code, so the conventions agree with every
//! renderer built on the same parser.

use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// One GFM table cell: its byte range (the text between the pipes) and its
/// place in the table.
#[derive(Debug, Clone)]
pub(crate) struct Cell {
    pub range: Range<usize>,
    /// The cell belongs to the header row.
    pub header: bool,
    /// The cell is the first one of its row.
    pub first: bool,
}

/// The parts of a document that the line-level conventions must know about.
pub(crate) struct Layout {
    code: Vec<Range<usize>>,
    code_blocks: Vec<Range<usize>>,
    comments: Vec<Range<usize>>,
    comment_blocks: Vec<Range<usize>>,
    cells: Vec<Cell>,
    tables: Vec<Range<usize>>,
}

impl Layout {
    /// Parse `text` with the suite's Markdown options and record code
    /// regions, HTML comments, tables and table cells (byte ranges).
    pub fn of(text: &str) -> Layout {
        let mut code = Vec::new();
        let mut code_blocks = Vec::new();
        let mut comments = Vec::new();
        let mut comment_blocks = Vec::new();
        let mut cells = Vec::new();
        let mut tables = Vec::new();
        let mut in_head = false;
        let mut first_in_row = false;
        for (event, range) in Parser::new_ext(text, crate::md_options()).into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    code_blocks.push(range.clone());
                    code.push(range);
                }
                Event::Code(_) => code.push(range),
                Event::Start(Tag::HtmlBlock) => {
                    if text[range.clone()].trim_start().starts_with("<!--") {
                        comment_blocks.push(range.clone());
                        comments.push(range);
                    }
                }
                Event::InlineHtml(ref html) if html.starts_with("<!--") => comments.push(range),
                Event::Start(Tag::Table(_)) => tables.push(range),
                Event::Start(Tag::TableHead) => {
                    in_head = true;
                    first_in_row = true;
                }
                Event::End(TagEnd::TableHead) => in_head = false,
                Event::Start(Tag::TableRow) => first_in_row = true,
                Event::Start(Tag::TableCell) => {
                    cells.push(Cell { range, header: in_head, first: first_in_row });
                    first_in_row = false;
                }
                _ => {}
            }
        }
        code.sort_by_key(|r| r.start);
        comments.sort_by_key(|r| r.start);
        Layout { code, code_blocks, comments, comment_blocks, cells, tables }
    }

    /// True if byte `pos` lies in a code block or a code span.
    pub fn is_code(&self, pos: usize) -> bool {
        within(&self.code, pos)
    }

    /// True if byte `pos` lies in an HTML comment.
    pub fn is_comment(&self, pos: usize) -> bool {
        within(&self.comments, pos)
    }

    /// True if conventions do not apply at byte `pos` (code or comment).
    pub fn is_excluded(&self, pos: usize) -> bool {
        self.is_code(pos) || self.is_comment(pos)
    }

    /// True if byte `pos` lies in a code block or a block-level HTML
    /// comment: a line starting there is not a line of text at all.
    pub fn is_block_excluded(&self, pos: usize) -> bool {
        within(&self.code_blocks, pos) || within(&self.comment_blocks, pos)
    }

    /// The HTML comments of the document (byte ranges, in order).
    pub fn comments(&self) -> &[Range<usize>] {
        &self.comments
    }

    /// Index of the table cell containing byte `pos`, if any.
    pub fn cell_index(&self, pos: usize) -> Option<usize> {
        let idx = self.cells.partition_point(|c| c.range.start <= pos);
        (idx > 0 && pos < self.cells[idx - 1].range.end).then(|| idx - 1)
    }

    /// The table cell with the given index.
    pub fn cell(&self, index: usize) -> &Cell {
        &self.cells[index]
    }

    /// The table that starts within `range` (a table begins at its header
    /// row, so this asks "does a table start on this line?").
    pub fn table_starting_in(&self, range: Range<usize>) -> Option<Range<usize>> {
        self.tables.iter().find(|t| range.contains(&t.start)).cloned()
    }
}

/// True if `pos` falls inside one of the sorted, non-overlapping `ranges`.
fn within(ranges: &[Range<usize>], pos: usize) -> bool {
    let idx = ranges.partition_point(|r| r.start <= pos);
    idx > 0 && pos < ranges[idx - 1].end
}

/// A document split into lines, each with its byte offset. A trailing `\r`
/// stays out of `text` but is remembered, so line-based rewrites can keep a
/// file's CRLF endings intact.
pub(crate) struct SourceLine<'a> {
    /// Byte offset of the line start in the document.
    pub start: usize,
    /// The line without its `\r\n` / `\n` terminator.
    pub text: &'a str,
    /// The terminator as found: `"\r\n"`, `"\n"`, or `""` for a last line
    /// without one.
    pub ending: &'static str,
}

impl SourceLine<'_> {
    /// The terminator to use after a line inserted next to this one.
    pub fn newline(&self) -> &'static str {
        if self.ending == "\r\n" {
            "\r\n"
        } else {
            "\n"
        }
    }

    /// True if the line holds only whitespace.
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Split `text` into [`SourceLine`]s. A final line without a terminator is
/// included; an empty remainder after the last `\n` is not.
pub(crate) fn lines(text: &str) -> Vec<SourceLine<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = text[start..].find('\n').map(|i| start + i);
        let raw = &text[start..end.unwrap_or(text.len())];
        let (body, ending) = match (raw.strip_suffix('\r'), end) {
            (Some(b), Some(_)) => (b, "\r\n"),
            (_, Some(_)) => (raw, "\n"),
            (_, None) => (raw, ""),
        };
        out.push(SourceLine { start, text: body, ending });
        start = match end {
            Some(e) => e + 1,
            None => text.len(),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_spans_and_blocks_are_code() {
        let text = "a `x ⟦y⟧` b\n\n```md\n> 🔀 ⟦KONIEC⟧\n```\n\n    indented\n";
        let layout = Layout::of(text);
        assert!(layout.is_code(text.find("⟦y").unwrap()));
        assert!(layout.is_code(text.find("> 🔀").unwrap()));
        assert!(layout.is_code(text.find("indented").unwrap()));
        assert!(layout.is_block_excluded(text.find("indented").unwrap()));
        assert!(!layout.is_block_excluded(text.find("⟦y").unwrap()));
        assert!(!layout.is_code(0));
    }

    #[test]
    fn test_comments_are_recorded() {
        let text = "<!-- block\ncomment -->\n\ntext <!-- inline --> more\n";
        let layout = Layout::of(text);
        assert!(layout.is_comment(text.find("block").unwrap()));
        assert!(layout.is_comment(text.find("inline").unwrap()));
        assert!(!layout.is_comment(text.find("more").unwrap()));
        assert_eq!(layout.comments().len(), 2);
    }

    #[test]
    fn test_table_cells_know_header_and_first() {
        let text = "| H1 | H2 |\n|---|---|\n| a | b |\n";
        let layout = Layout::of(text);
        let h1 = layout.cell(layout.cell_index(text.find("H1").unwrap()).unwrap());
        assert!(h1.header && h1.first);
        let a = layout.cell(layout.cell_index(text.find(" a ").unwrap() + 1).unwrap());
        assert!(!a.header && a.first);
        let b = layout.cell(layout.cell_index(text.find(" b ").unwrap() + 1).unwrap());
        assert!(!b.header && !b.first);
        assert!(layout.table_starting_in(0..11).is_some());
        assert!(layout.table_starting_in(12..20).is_none());
    }

    #[test]
    fn test_lines_keep_offsets_and_crlf() {
        let text = "one\r\ntwo\n\nthree";
        let ls = lines(text);
        assert_eq!(ls.len(), 4);
        assert_eq!((ls[0].text, ls[0].ending, ls[0].start), ("one", "\r\n", 0));
        assert_eq!((ls[1].text, ls[1].ending, ls[1].start), ("two", "\n", 5));
        assert!(ls[2].is_blank());
        assert_eq!((ls[3].text, ls[3].ending), ("three", ""));
        assert_eq!(ls[0].newline(), "\r\n");
        assert_eq!(ls[3].newline(), "\n");
        assert!(lines("").is_empty());
        assert_eq!(lines("x\n").len(), 1);
    }
}
