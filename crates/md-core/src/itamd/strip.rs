//! Remove directives without evaluating them: every branch stays. This is
//! what a converter that does not compile does to a source — the result is
//! still a source's text, not a deliverable, and tools say so.

use super::lex::{self, LineKind, Token};

/// Remove all ITA-MD directives from `source`, keeping every branch:
///
/// * a block or table directive line goes together with the one blank line
///   after it, so the layout equals the source without directives;
/// * inline and row directives are cut out of their lines;
/// * code blocks, code spans and HTML comments are left untouched, and so
///   is anything that is not a well-formed directive (a malformed 🔀 line
///   stays a working note; a stray bracket stays text).
///
/// Line endings (LF or CRLF) are preserved; a leading BOM is dropped.
pub fn strip_directives(source: &str) -> String {
    strip_counted(source).0
}

/// [`strip_directives`] plus the number of directives removed.
pub(crate) fn strip_counted(source: &str) -> (String, usize) {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let scan = lex::scan(source);
    let mut out = String::with_capacity(source.len());
    let mut removed = 0;
    let mut skip_blank = false;
    for (line, kind) in scan.lines.iter().zip(&scan.kinds) {
        if std::mem::take(&mut skip_blank) && line.is_blank() {
            continue;
        }
        match kind {
            LineKind::Block { .. } => {
                removed += 1;
                skip_blank = true;
                continue;
            }
            LineKind::Text { tokens } if tokens.iter().any(|t| matches!(t, Token::Directive(_))) => {
                let mut pos = 0;
                for t in tokens {
                    if let Token::Directive(d) = t {
                        out.push_str(&line.text[pos..d.span.start]);
                        pos = d.span.end;
                        removed += 1;
                    }
                }
                out.push_str(&line.text[pos..]);
            }
            _ => out.push_str(line.text),
        }
        out.push_str(line.ending);
    }
    (out, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_lines_go_with_one_blank_line() {
        let src = "Przed.\n\n> 🔀 ⟦JEŻELI tier >= T4⟧\n\n6. Punkt.\n> 🔀 ⟦KONIEC⟧\n\n\nPo.\n";
        assert_eq!(strip_directives(src), "Przed.\n\n6. Punkt.\n\nPo.\n");
    }

    #[test]
    fn test_inline_and_row_directives_keep_all_branches() {
        let src = "Określa ⟦JEŻELI tier >= T4⟧Załączniki A i D⟦W PRZECIWNYM RAZIE⟧Załącznik A⟦KONIEC⟧.\n\n| P | x |\n|---|---|\n| ⟦WIERSZ tier == T1⟧**T1** | a |\n";
        assert_eq!(
            strip_directives(src),
            "Określa Załączniki A i DZałącznik A.\n\n| P | x |\n|---|---|\n| **T1** | a |\n"
        );
    }

    #[test]
    fn test_table_directive_and_crlf() {
        let src = "> 🔀 ⟦TABELA · kolumny: poziom⟧\r\n\r\n| P | T1 |\r\n|---|---|\r\n| a | b |\r\n";
        assert_eq!(strip_directives(src), "| P | T1 |\r\n|---|---|\r\n| a | b |\r\n");
    }

    #[test]
    fn test_code_and_broken_text_untouched() {
        let src = "`⟦KONIEC⟧`\n\n```\n> 🔀 ⟦JEŻELI x⟧\n\n```\n\n> 🔀 notatka\n\nzły ⟦JEZELI x⟧ nawias\n";
        assert_eq!(strip_directives(src), src);
    }

    #[test]
    fn test_counts_and_bom() {
        let (text, n) = strip_counted("\u{feff}a⟦JEŻELI x⟧b⟦KONIEC⟧\n> 🔀 ⟦KONIEC⟧\n");
        assert_eq!((text.as_str(), n), ("ab\n", 3));
        assert_eq!(strip_counted("plain\n").1, 0);
        // A last line without a newline stays without one.
        assert_eq!(strip_directives("a⟦JEŻELI x⟧b⟦KONIEC⟧"), "ab");
    }
}
